//! Abstract pipeline tests that use generic stage/mode/role names.
//!
//! These tests verify the pipeline machinery independently of specific
//! naming conventions like "planning", "working", etc.
#![allow(dead_code)]

use std::collections::HashMap;

use indexmap::IndexMap;
use zbobr_api::{
    Pipeline, Signal, Stage, StageTransition, State,
    config::{PipelineConfig, Role, RoleDefinition, StageDefinition, WorkflowConfig},
    config_tools::ALL_TOOLS,
};
use zbobr_dispatcher::backend::TaskBackendExt;
use zbobr_utility::TomlOption;

use super::{abstract_scenarios, env::IntegrationTestEnv};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct StageDef {
    name: &'static str,
    role: &'static str,
    pipeline: &'static str,
    on_success: Option<StageTransition>,
    on_failure: Option<StageTransition>,
    on_intermediate: Option<StageTransition>,
}

impl StageDef {
    fn new(name: &'static str, role: &'static str, pipeline: &'static str) -> Self {
        Self {
            name,
            role,
            pipeline,
            on_success: Default::default(),
            on_failure: Default::default(),
            on_intermediate: Default::default(),
        }
    }
}

/// Build a WorkflowConfig from a list of stage definitions.
///
/// Stages are added to their respective pipelines in the order they appear.
/// Optional `roles` map allows specifying tool lists for roles.
fn build_workflow_with_roles(
    stages: Vec<StageDef>,
    roles: IndexMap<Role, RoleDefinition>,
) -> WorkflowConfig {
    let mut pipeline_stages: HashMap<Pipeline, IndexMap<Stage, StageDefinition>> = HashMap::new();

    for s in stages {
        pipeline_stages
            .entry(Pipeline::from(s.pipeline))
            .or_default()
            .insert(
                Stage::from(s.name),
                StageDefinition {
                    role: Some(s.role.to_string().into()).into(),
                    tool: Some("mcp-tester".to_string().into()).into(),
                    on_success: s.on_success.into(),
                    on_failure: s.on_failure.into(),
                    on_intermediate: s.on_intermediate.into(),
                    ..Default::default()
                },
            );
    }

    let mut pipelines: IndexMap<Pipeline, PipelineConfig> = IndexMap::new();
    for (pipeline_name, stages) in pipeline_stages {
        pipelines.insert(pipeline_name, PipelineConfig { stages: Some(stages.into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()) });
    }

    WorkflowConfig {
        prompts: None,
        pipelines: Some(pipelines.into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        roles: Some(roles.into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        ..Default::default()
    }
}

fn role_with_all_tools() -> RoleDefinition {
    RoleDefinition {
        mcp: Some(ALL_TOOLS.to_vec()),
        prompts: Default::default(),
        tool: Some("mcp-tester".to_string().into()).into(),
    }
}

fn build_workflow(stages: Vec<StageDef>) -> WorkflowConfig {
    let roles = stages
        .iter()
        .map(|s| (s.role.to_string().into(), role_with_all_tools()))
        .collect();
    build_workflow_with_roles(stages, roles)
}

fn scenarios_map(entries: Vec<(&str, String)>) -> HashMap<String, String> {
    entries
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

async fn git_in(dir: &std::path::Path, args: &[&str]) {
    let status = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .await
        .unwrap();
    assert!(
        status.success(),
        "git {:?} failed in {}",
        args,
        dir.display()
    );
}

async fn write_and_commit(dir: &std::path::Path, file: &str, content: &str, msg: &str) {
    tokio::fs::write(dir.join(file), content).await.unwrap();
    git_in(dir, &["add", file]).await;
    git_in(dir, &["commit", "-m", msg]).await;
}

// ===========================================================================
// Test 1: All MCP operations in a single stage
// ===========================================================================

pub async fn run_all_mcp_tools(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_mcp_tools").await;
    let task_id = env
        .create_task("Test task", "Test task description", "READY")
        .await;
    // Pre-set work branch so the repo backend can set up the workspace
    let work_branch = format!("zbobr_fix-{task_id}-mcp");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("alpha", "alpha", "main")]);

    let scenarios = scenarios_map(vec![(
        "alpha",
        abstract_scenarios::all_mcp_tools_scenario(&repo_path.to_string_lossy()),
    )]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, State::Done, "Should complete as DONE");
}

// ===========================================================================
// Test 2: Sequential stage advancement within a pipeline
// ===========================================================================

pub async fn run_stage_transfer(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_transfer").await;
    let task_id = env
        .create_task("Transfer test", "Transfer test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-transfer");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![
        StageDef::new("first", "role_a", "main"),
        StageDef::new("second", "role_b", "main"),
    ]);

    let scenarios = scenarios_map(vec![
        ("role_a", abstract_scenarios::report_and_finish_scenario()),
        ("role_b", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // First call runs "first" stage → report_success → auto-advance to second
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::go("second")),
        "Success in first stage should advance to go_second"
    );
    assert_eq!(task.state, State::Pending(Pipeline::Main));

    // Run to completion: second stage + return resolution
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, State::Done, "Should complete after both stages");
}

// ===========================================================================
// Test 5: Automatic conflict pipeline activation
// ===========================================================================

pub async fn run_auto_conflict(env: &IntegrationTestEnv) {
    if env.target_repo.is_some() {
        eprintln!(
            "[{}] Skipping run_auto_conflict: requires local repo",
            env.name()
        );
        return;
    }

    let repo_path = env
        .local_repo_path
        .as_ref()
        .expect("fs integration env should expose local_repo_path")
        .clone();
    let work_branch = "zbobr_conflict-detect-abstract";

    // Create work branch with a commit
    git_in(&repo_path, &["checkout", "-b", work_branch]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "work version\n",
        "Work change",
    )
    .await;
    // Diverge main
    git_in(&repo_path, &["checkout", "main"]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "main version\n",
        "Main change",
    )
    .await;

    let task_id = env
        .create_task("Conflict test", "Conflict test description", "READY")
        .await;
    env.update_task_branches(task_id, work_branch).await;

    let workflow = build_workflow(vec![
        StageDef::new("work", "role_work", "main"),
        StageDef::new("resolve", "role_resolve", "merge"),
    ]);

    let scenarios = scenarios_map(vec![
        (
            "role_work",
            abstract_scenarios::report_and_finish_scenario(),
        ),
        (
            "role_resolve",
            abstract_scenarios::report_and_finish_scenario(),
        ),
    ]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::call("merge")),
        "Diverged work branch should trigger call_merge"
    );
    assert_eq!(
        task.stack.len(),
        1,
        "Stack should have the caller stage pushed"
    );
    assert_eq!(task.stack[0].pipeline, zbobr_api::Pipeline::Main);
    assert_eq!(
        task.stack[0].signal,
        Signal::go("work"),
        "Stack entry should have signal go_work (re-run interrupted stage)"
    );

    // Step 2: run the conflict handler (merging/resolve) → return
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some(Signal::Return));
    assert_eq!(task.state, State::Pending(Pipeline::Merge));

    // Step 3: process return → stack pop → go_work in main pipeline
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::go("work")),
        "After return from conflict, signal should be go_work (popped from stack)"
    );
    assert_eq!(task.state, State::Pending(Pipeline::Main));
    assert!(task.stack.is_empty(), "Stack should be empty after pop");
}

// ===========================================================================
// Test 6: Going to PAUSE via stop_with_error
// ===========================================================================

pub async fn run_pause_on_error(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_pause").await;
    let task_id = env
        .create_task("Pause test", "Pause test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-pause");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("work", "role_err", "main")]);

    let scenarios = scenarios_map(vec![(
        "role_err",
        abstract_scenarios::stop_with_error_scenario(),
    )]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.go_pause, "stop_with_error should set pause flag");
    assert_eq!(
        task.state,
        State::Pending(Pipeline::Main),
        "Should be pending, not DONE"
    );
    assert_eq!(
        task.signal,
        Some(Signal::go("work")),
        "Pause should set signal to re-run the stage"
    );

    assert!(
        task.status
            .as_ref()
            .map(|e| e.contains("Something went wrong"))
            .unwrap_or(false),
        "Error message should be stored in task.status"
    );
}

// ===========================================================================
// Test 7: READY handling — dispatch start stage, run to DONE
// ===========================================================================

pub async fn run_ready_dispatch(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_ready").await;
    let task_id = env
        .create_task("Ready test", "Ready test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-ready");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("start", "role_start", "main")]);

    let scenarios = scenarios_map(vec![(
        "role_start",
        abstract_scenarios::report_and_finish_scenario(),
    )]);

    // Start from READY, run stage, then resolve return → DONE
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state,
        State::Done,
        "READY task should dispatch and complete"
    );
}

// ===========================================================================
// Test 8: Failure causes return_failure (sequential model)
// ===========================================================================

pub async fn run_signal_transitions(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_signals").await;
    let task_id = env
        .create_task("Signal test", "Signal test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-signals");
    env.update_task_branches(task_id, &work_branch).await;

    let mut pipelines: IndexMap<Pipeline, PipelineConfig> = IndexMap::new();
    pipelines.insert(
        Pipeline::Main,
        PipelineConfig {
            stages: Some(IndexMap::from([
                (
                    Stage::from("check"),
                    StageDefinition {
                        role: Some("role_check".to_string().into()).into(),
                        tool: Some("mcp-tester".to_string().into()).into(),
                        ..Default::default()
                    },
                ),
                (
                    Stage::from("finish"),
                    StageDefinition {
                        role: Some("role_finish".to_string().into()).into(),
                        tool: Some("mcp-tester".to_string().into()).into(),
                        ..Default::default()
                    },
                ),
            ]).into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        },
    );
    let workflow = WorkflowConfig {
        prompts: None,
        pipelines: Some(pipelines.into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        roles: Some(IndexMap::from([
            ("role_check".to_string().into(), TomlOption::Value(role_with_all_tools())),
            ("role_finish".to_string().into(), TomlOption::Value(role_with_all_tools())),
        ])),
        ..Default::default()
    };

    // First run: failure → return_failure → root pipeline pauses
    let scenarios_reject = scenarios_map(vec![
        ("role_check", abstract_scenarios::report_failure_scenario()),
        (
            "role_finish",
            abstract_scenarios::report_and_finish_scenario(),
        ),
    ]);
    env.run_pipeline(task_id, &workflow, &scenarios_reject)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::ReturnFailure),
        "report_failure should produce return_failure signal"
    );

    // Process the return_failure: root pipeline failed → paused (no caller to return to)
    env.continue_pipeline(task_id, &workflow, &scenarios_reject)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, State::Pending(Pipeline::Main));
    assert!(task.go_pause, "Should be paused at root on failure");
    assert_eq!(
        task.signal,
        Some(Signal::go("check")),
        "Root failure should set signal to pipeline's first stage for resume"
    );
}

// ===========================================================================
// Test 9: PAUSE via stop_with_question
// ===========================================================================

pub async fn run_pause_on_ask_user(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_ask").await;
    let task_id = env
        .create_task("Ask test", "Ask test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-ask");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("work", "role_ask", "main")]);

    let scenarios = scenarios_map(vec![(
        "role_ask",
        abstract_scenarios::stop_with_question_scenario(),
    )]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.go_pause, "stop_with_question should set pause flag");
    assert_eq!(
        task.signal,
        Some(Signal::go("work")),
        "Pause should set signal to re-run the stage"
    );
}

// ===========================================================================
// Test 10: Undefined identity — stage runs with task_dir
// ===========================================================================

pub async fn run_auto_undefined(env: &IntegrationTestEnv) {
    // Create task WITHOUT identity fields — no routing params
    let task_id = env
        .create_task("Undefined test", "Undefined test description", "READY")
        .await;
    // DO NOT set branches — identity is undefined

    // Stage has no prompt files, so no {work_branch}/{destination_branch} placeholders.
    // With undefined identity the stage should run normally using task_dir.
    let workflow = build_workflow(vec![StageDef::new("working", "role_work", "main")]);

    let scenarios = scenarios_map(vec![(
        "role_work",
        abstract_scenarios::report_and_finish_scenario(),
    )]);

    // The stage should execute — identity undefined is no longer a dispatch trigger.
    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    // report_success on single-stage main pipeline → Return signal → Done
    assert!(
        task.stack.is_empty(),
        "Stack should be empty after successful single-stage run"
    );
}

// ===========================================================================
// Test 12: Call stage invokes a sub-pipeline and advances on return
// ===========================================================================

pub async fn run_call_stage(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_call_stage").await;
    let task_id = env
        .create_task("Call stage test", "Call stage test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-call");
    env.update_task_branches(task_id, &work_branch).await;

    // Build workflow:
    //   main:  call_sub → finish
    //   sub:   work (reports success)
    //   init:  preparing (required)
    //   merge: merging   (required)
    let mut pipelines: IndexMap<Pipeline, PipelineConfig> = IndexMap::new();
    pipelines.insert(
        Pipeline::Main,
        PipelineConfig {
            stages: Some(IndexMap::from([
                (
                    "call_sub".into(),
                    StageDefinition {
                        call: Some("sub".into()).into(),
                        ..Default::default()
                    },
                ),
                (
                    "finish".into(),
                    StageDefinition {
                        role: Some("role_finish".into()).into(),
                        tool: Some("mcp-tester".to_string().into()).into(),
                        ..Default::default()
                    },
                ),
            ]).into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        },
    );
    pipelines.insert(
        Pipeline::from("sub"),
        PipelineConfig {
            stages: Some(IndexMap::from([(
                "work".into(),
                StageDefinition {
                    role: Some("role_work".into()).into(),
                    tool: Some("mcp-tester".to_string().into()).into(),
                    ..Default::default()
                },
            )]).into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        },
    );
    pipelines.insert(
        Pipeline::Merge,
        PipelineConfig {
            stages: Some(IndexMap::from([(
                "merging".into(),
                StageDefinition {
                    role: Some("role_merge".into()).into(),
                    tool: Some("mcp-tester".to_string().into()).into(),
                    ..Default::default()
                },
            )]).into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        },
    );
    let workflow = WorkflowConfig {
        prompts: None,
        pipelines: Some(pipelines.into_iter().map(|(k,v)| (k, TomlOption::Value(v))).collect()),
        roles: Some(IndexMap::from([
            ("role_work".to_string().into(), TomlOption::Value(role_with_all_tools())),
            ("role_finish".to_string().into(), TomlOption::Value(role_with_all_tools())),
            ("role_merge".to_string().into(), TomlOption::Value(role_with_all_tools())),
        ])),
        ..Default::default()
    };

    let scenarios = scenarios_map(vec![
        (
            "role_work",
            abstract_scenarios::report_and_finish_scenario(),
        ),
        (
            "role_finish",
            abstract_scenarios::report_and_finish_scenario(),
        ),
    ]);

    // Step 1: process_task from READY — hits call stage, pushes stack, emits call_sub
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::call("sub")),
        "Call stage should emit call_sub signal"
    );
    assert_eq!(task.state, State::Pending(Pipeline::Main));
    assert_eq!(task.stack.len(), 1, "Stack should have one entry");
    assert_eq!(task.stack[0].pipeline, zbobr_api::Pipeline::Main);
    assert_eq!(
        task.stack[0].signal,
        Signal::go("finish"),
        "Return signal should advance to next stage"
    );

    // Step 2..N: run to completion through sub-pipeline → return → finish → DONE
    env.run_to_completion(task_id, &workflow, &scenarios, 10)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state,
        State::Done,
        "Task should complete after call stage returns and finish stage runs"
    );
    assert!(
        task.stack.is_empty(),
        "Stack should be empty after completion"
    );
}

// ===========================================================================
// Test 13: Pause flag converts to PAUSE state with stack push
// ===========================================================================

pub async fn run_pause_state_conversion(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_pause_conv").await;
    let task_id = env
        .create_task(
            "Pause conversion test",
            "Pause conversion test description",
            "READY",
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-pause-conv");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("work", "role_err", "main")]);

    let scenarios = scenarios_map(vec![(
        "role_err",
        abstract_scenarios::stop_with_error_scenario(),
    )]);

    // Step 1: run_pipeline → stage runs → stop_with_error → pause=true
    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.go_pause, "stop_with_error should set pause flag");
    assert_eq!(task.state, State::Pending(Pipeline::Main));
    assert_eq!(task.signal, Some(Signal::go("work")));

    // Step 2: continue_pipeline → centralized handler converts pause flag to PAUSE state
    env.continue_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(!task.go_pause, "Pause flag should be cleared");
    assert_eq!(task.state, State::Pause, "State should be PAUSE");
    assert!(task.signal.is_none(), "Signal should be cleared");
    assert_eq!(task.stack.len(), 1, "Stack should have one entry");
    assert_eq!(task.stack[0].pipeline, zbobr_api::Pipeline::Main);
    assert_eq!(
        task.stack[0].signal,
        Signal::go("work"),
        "Stack entry should save signal to re-run the stage"
    );
}

// ===========================================================================
// Test 14: Full pause → READY → resume cycle
// ===========================================================================

pub async fn run_pause_resume_cycle(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_pause_resume").await;
    let task_id = env
        .create_task(
            "Pause resume test",
            "Pause resume test description",
            "READY",
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-pause-resume");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("work", "role_work", "main")]);

    // Step 1: run stage with stop_with_error → pause
    let err_scenarios = scenarios_map(vec![(
        "role_work",
        abstract_scenarios::stop_with_error_scenario(),
    )]);
    env.run_pipeline(task_id, &workflow, &err_scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.go_pause, "Should be paused after stop_with_error");

    // Step 2: convert pause to PAUSE state
    env.continue_pipeline(task_id, &workflow, &err_scenarios)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, State::Pause);
    assert_eq!(task.stack.len(), 1);

    // Step 3: simulate user unpause by setting state to READY
    env.zbobr
        .task_backend()
        .set_task_state(task_id, zbobr_api::State::Ready)
        .await
        .unwrap();

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, State::Ready);
    assert_eq!(task.stack.len(), 1, "Stack should still have the entry");

    // Step 4: process READY → pops stack, restores pipeline/signal
    let success_scenarios = scenarios_map(vec![(
        "role_work",
        abstract_scenarios::report_and_finish_scenario(),
    )]);
    env.continue_pipeline(task_id, &workflow, &success_scenarios)
        .await;

    // After READY handler: state=Pending(main), signal=Go(work)
    // Then process_task resolves and runs the stage with success scenario.
    // After the stage succeeds (single stage → Return → Done at root → finish)
    // Let it run to completion.
    env.run_to_completion(task_id, &workflow, &success_scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state,
        State::Done,
        "Task should complete after pause/resume cycle"
    );
    assert!(
        task.stack.is_empty(),
        "Stack should be empty after completion"
    );
}

// ===========================================================================
// Test 15: READY with empty stack — fresh start from main pipeline
// ===========================================================================

pub async fn run_ready_fresh_start(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_fresh").await;
    let task_id = env
        .create_task("Fresh start test", "Fresh start test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-fresh");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("start", "role_start", "main")]);

    let scenarios = scenarios_map(vec![(
        "role_start",
        abstract_scenarios::report_and_finish_scenario(),
    )]);

    // READY with empty stack should start from main pipeline's first stage
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state,
        State::Done,
        "READY with empty stack should start fresh and complete"
    );
}

// ===========================================================================
// Test 16: Stage with on_success.pause pauses after success, then advances
// ===========================================================================

pub async fn run_stage_pause_on_success(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_stage_pause").await;
    let task_id = env
        .create_task("Stage pause test", "Stage pause test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-stage-pause");
    env.update_task_branches(task_id, &work_branch).await;

    // Two stages: stage_a has on_success = { pause = true }, stage_b is normal
    let workflow = build_workflow(vec![
        StageDef {
            on_success: Some(StageTransition::pause()).into(),
            ..StageDef::new("stage_a", "role_a", "main")
        },
        StageDef::new("stage_b", "role_b", "main"),
    ]);

    let scenarios = scenarios_map(vec![
        ("role_a", abstract_scenarios::report_and_finish_scenario()),
        ("role_b", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // Step 1: Run pipeline — stage_a succeeds but on_success has pause: true
    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(
        task.go_pause,
        "Stage with on_success.pause should set pause flag after report_success"
    );
    assert_eq!(
        task.signal,
        Some(Signal::go("stage_b")),
        "Pause signal should point to NEXT stage, not current"
    );
    assert_eq!(task.state, State::Pending(Pipeline::Main));

    // Step 2: Convert pause to PAUSE state
    env.continue_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, State::Pause);
    assert!(!task.go_pause, "Pause flag should be cleared");
    assert!(task.signal.is_none(), "Signal should be cleared");
    assert_eq!(task.stack.len(), 1, "Stack should have one entry");
    assert_eq!(
        task.stack[0].signal,
        Signal::go("stage_b"),
        "Stack should save signal to advance to next stage"
    );

    // Step 3: User unpauses by setting state to READY
    env.zbobr
        .task_backend()
        .set_task_state(task_id, zbobr_api::State::Ready)
        .await
        .unwrap();

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, State::Ready);
    assert_eq!(task.stack.len(), 1);

    // Step 4: Resume and run to completion (stage_b runs, then DONE)
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state,
        State::Done,
        "Task should complete after pause/resume with stage advance"
    );
    assert!(
        task.stack.is_empty(),
        "Stack should be empty after completion"
    );
}

// ===========================================================================
// Test 17: Graceful pause when runner.run() fails before MCP (empty description)
// ===========================================================================

pub async fn run_pause_on_runner_error(env: &IntegrationTestEnv) {
    let _repo_path = env.create_git_repo("repo_runner_err").await;
    // Create task with EMPTY description — this triggers the pre-flight check error
    // inside CliStageRunner::run() before the MCP server is started.
    let task_id = env.create_task("Runner error test", "", "READY").await;
    let work_branch = format!("zbobr_fix-{task_id}-runner-err");
    env.update_task_branches(task_id, &work_branch).await;

    let workflow = build_workflow(vec![StageDef::new("work", "role_work", "main")]);

    // Scenarios are irrelevant — the stage never reaches MCP.
    let scenarios = scenarios_map(vec![(
        "role_work",
        abstract_scenarios::report_and_finish_scenario(),
    )]);

    // Step 1: run_pipeline → runner.run() errors on empty description → graceful pause
    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(
        task.go_pause,
        "runner.run() error should set pause flag for graceful pause"
    );
    assert_eq!(
        task.state,
        State::Running(Pipeline::Main, Stage::from("work")),
        "State should still be Running when pause flag is set"
    );
    assert_eq!(
        task.signal,
        Some(Signal::go("work")),
        "Signal should be set to re-run the failed stage"
    );
    assert!(
        task.status
            .as_ref()
            .map(|s| s.contains("no description"))
            .unwrap_or(false),
        "Error message should mention missing description; got: {:?}",
        task.status
    );

    // Step 2: continue_pipeline → centralized handler converts pause flag to PAUSE state
    env.continue_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(!task.go_pause, "Pause flag should be cleared");
    assert_eq!(task.state, State::Pause, "State should be PAUSE");
    assert!(task.signal.is_none(), "Signal should be cleared");
    assert_eq!(task.stack.len(), 1, "Stack should have one entry");
    assert_eq!(task.stack[0].pipeline, zbobr_api::Pipeline::Main);
    assert_eq!(
        task.stack[0].signal,
        Signal::go("work"),
        "Stack entry should save signal to re-run the stage"
    );
}
