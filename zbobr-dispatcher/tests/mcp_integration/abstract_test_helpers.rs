//! Abstract pipeline tests that use generic stage/mode/role names.
//!
//! These tests verify the pipeline machinery independently of specific
//! naming conventions like "planning", "working", etc.
#![allow(dead_code)]

use std::collections::HashMap;

use indexmap::IndexMap;
use zbobr_api::{
    Pipeline, Signal, Stage,
    config::{PipelineConfig, RoleDefinition, StageDefinition, WorkflowConfig},
};
use zbobr_dispatcher::task::Tool;
use zbobr_executor_mcp_tester;

use super::{abstract_scenarios, env::IntegrationTestEnv};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct StageDef {
    name: &'static str,
    role: &'static str,
    pipeline: &'static str,
}

/// Build a WorkflowConfig from a list of stage definitions.
///
/// Stages are added to their respective pipelines in the order they appear.
/// Optional `roles` map allows specifying tool lists for roles.
fn build_workflow_with_roles(
    stages: Vec<StageDef>,
    roles: HashMap<String, RoleDefinition>,
) -> WorkflowConfig {
    let mut pipeline_stages: HashMap<Pipeline, IndexMap<Stage, StageDefinition>> = HashMap::new();

    for s in stages {
        pipeline_stages
            .entry(Pipeline::from(s.pipeline))
            .or_default()
            .insert(
                Stage::from(s.name),
                StageDefinition {
                    role: Some(s.role.to_string()),
                    tool: Some(Tool::McpTester),
                    ..Default::default()
                },
            );
    }

    let mut pipelines: HashMap<Pipeline, PipelineConfig> = HashMap::new();
    for (pipeline_name, stages) in pipeline_stages {
        pipelines.insert(pipeline_name, PipelineConfig { stages });
    }

    WorkflowConfig {
        prompts_dir: None,
        pipelines,
        roles,
    }
}

fn build_workflow(stages: Vec<StageDef>) -> WorkflowConfig {
    build_workflow_with_roles(stages, Default::default())
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
    // Pre-set routing params so workspace preparation succeeds
    let work_branch = format!("zbobr_fix-{task_id}-mcp");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    let workflow = build_workflow(vec![StageDef {
        name: "alpha",
        role: "alpha",
        pipeline: "main",
    }]);

    let scenarios = scenarios_map(vec![(
        "alpha",
        abstract_scenarios::all_mcp_tools_scenario(&repo_path.to_string_lossy()),
    )]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Should complete as DONE");
}

// ===========================================================================
// Test 2: Sequential stage advancement within a pipeline
// ===========================================================================

pub async fn run_stage_transfer(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_transfer").await;
    let task_id = env
        .create_task("Transfer test", "Transfer test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-transfer");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    let workflow = build_workflow(vec![
        StageDef {
            name: "first",
            role: "role_a",
            pipeline: "main",
        },
        StageDef {
            name: "second",
            role: "role_b",
            pipeline: "main",
        },
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
    assert_eq!(task.state, "main_PENDING");

    // Run to completion: second stage + return resolution
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Should complete after both stages");
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

    let repo_path = env.create_git_repo("repo_auto_conflict").await;
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
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", work_branch)
        .await;

    let workflow = build_workflow(vec![
        StageDef {
            name: "work",
            role: "role_work",
            pipeline: "main",
        },
        StageDef {
            name: "resolve",
            role: "role_resolve",
            pipeline: "merge",
        },
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
    assert_eq!(task.state, "merge_PENDING");

    // Step 3: process return → stack pop → go_work in main pipeline
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::go("work")),
        "After return from conflict, signal should be go_work (popped from stack)"
    );
    assert_eq!(task.state, "main_PENDING");
    assert!(task.stack.is_empty(), "Stack should be empty after pop");
}

// ===========================================================================
// Test 6: Going to PAUSE via stop_with_error
// ===========================================================================

pub async fn run_pause_on_error(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_pause").await;
    let task_id = env
        .create_task("Pause test", "Pause test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-pause");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    let workflow = build_workflow(vec![StageDef {
        name: "work",
        role: "role_err",
        pipeline: "main",
    }]);

    let scenarios = scenarios_map(vec![(
        "role_err",
        abstract_scenarios::stop_with_error_scenario(),
    )]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.pause, "stop_with_error should set pause flag");
    assert_eq!(task.state, "main_PENDING", "Should be pending, not DONE");

    let comments = env.get_comments(task_id).await;
    assert!(
        comments
            .iter()
            .any(|c| c.text.contains("Something went wrong")),
        "Error comment should be recorded"
    );
}

// ===========================================================================
// Test 7: READY handling — dispatch start stage, run to DONE
// ===========================================================================

pub async fn run_ready_dispatch(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_ready").await;
    let task_id = env
        .create_task("Ready test", "Ready test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-ready");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    let workflow = build_workflow(vec![StageDef {
        name: "start",
        role: "role_start",
        pipeline: "main",
    }]);

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
        task.state, "DONE",
        "READY task should dispatch and complete"
    );
}

// ===========================================================================
// Test 8: Failure causes return_failure (sequential model)
// ===========================================================================

pub async fn run_signal_transitions(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_signals").await;
    let task_id = env
        .create_task("Signal test", "Signal test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-signals");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    let mut pipelines: HashMap<Pipeline, PipelineConfig> = HashMap::new();
    pipelines.insert(
        Pipeline::Main,
        PipelineConfig {
            stages: IndexMap::from([
                (
                    Stage::from("check"),
                    StageDefinition {
                        role: Some("role_check".to_string()),
                        tool: Some(Tool::McpTester),
                        ..Default::default()
                    },
                ),
                (
                    Stage::from("finish"),
                    StageDefinition {
                        role: Some("role_finish".to_string()),
                        tool: Some(Tool::McpTester),
                        ..Default::default()
                    },
                ),
            ]),
        },
    );
    let workflow = WorkflowConfig {
        prompts_dir: None,
        pipelines,
        roles: Default::default(),
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
    assert_eq!(task.state, "main_PENDING");
    assert!(task.pause, "Should be paused at root on failure");
}

// ===========================================================================
// Test 9: PAUSE via stop_with_question
// ===========================================================================

pub async fn run_pause_on_ask_user(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_ask").await;
    let task_id = env
        .create_task("Ask test", "Ask test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-ask");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    let workflow = build_workflow(vec![StageDef {
        name: "work",
        role: "role_ask",
        pipeline: "main",
    }]);

    let scenarios = scenarios_map(vec![(
        "role_ask",
        abstract_scenarios::stop_with_question_scenario(),
    )]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.pause, "stop_with_question should set pause flag");
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
    let workflow = build_workflow(vec![StageDef {
        name: "working",
        role: "role_work",
        pipeline: "main",
    }]);

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
    let repo_path = env.create_git_repo("repo_call_stage").await;
    let task_id = env
        .create_task("Call stage test", "Call stage test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-call");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // Build workflow:
    //   main:  call_sub → finish
    //   sub:   work (reports success)
    //   init:  preparing (required)
    //   merge: merging   (required)
    let mut pipelines: HashMap<Pipeline, PipelineConfig> = HashMap::new();
    pipelines.insert(
        Pipeline::Main,
        PipelineConfig {
            stages: IndexMap::from([
                (
                    "call_sub".into(),
                    StageDefinition {
                        call: Some("sub".into()),
                        ..Default::default()
                    },
                ),
                (
                    "finish".into(),
                    StageDefinition {
                        role: Some("role_finish".into()),
                        tool: Some(Tool::McpTester),
                        ..Default::default()
                    },
                ),
            ]),
        },
    );
    pipelines.insert(
        Pipeline::from("sub"),
        PipelineConfig {
            stages: IndexMap::from([(
                "work".into(),
                StageDefinition {
                    role: Some("role_work".into()),
                    tool: Some(Tool::McpTester),
                    ..Default::default()
                },
            )]),
        },
    );
    pipelines.insert(
        Pipeline::Merge,
        PipelineConfig {
            stages: IndexMap::from([(
                "merging".into(),
                StageDefinition {
                    role: Some("role_merge".into()),
                    tool: Some(Tool::McpTester),
                    ..Default::default()
                },
            )]),
        },
    );
    let workflow = WorkflowConfig {
        prompts_dir: None,
        pipelines,
        roles: Default::default(),
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
    assert_eq!(task.state, "main_PENDING");
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
        task.state, "DONE",
        "Task should complete after call stage returns and finish stage runs"
    );
    assert!(
        task.stack.is_empty(),
        "Stack should be empty after completion"
    );
}
