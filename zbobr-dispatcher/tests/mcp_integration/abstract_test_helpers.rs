//! Abstract pipeline tests that use generic stage/mode/role names.
//!
//! These tests verify the pipeline machinery independently of specific
//! naming conventions like "planning", "working", etc.
#![allow(dead_code)]

use std::collections::HashMap;

use zbobr_api::config::{PipelineConfig, RoleDefinition, StageDefinition, WorkflowConfig};
use zbobr_dispatcher::task::Tool;

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
/// The `order` field is derived from insertion order within each pipeline.
/// Optional `roles` map allows specifying tool lists for roles (needed for call_* tools).
fn build_workflow_with_roles(
    stages: Vec<StageDef>,
    roles: HashMap<String, RoleDefinition>,
) -> WorkflowConfig {
    // Collect stages per pipeline, preserving insertion order
    let mut pipeline_order: HashMap<String, Vec<String>> = HashMap::new();
    let mut pipeline_stages: HashMap<String, HashMap<String, StageDefinition>> = HashMap::new();

    for s in stages {
        pipeline_order
            .entry(s.pipeline.to_string())
            .or_default()
            .push(s.name.to_string());
        pipeline_stages
            .entry(s.pipeline.to_string())
            .or_default()
            .insert(
                s.name.to_string(),
                StageDefinition {
                    role: s.role.to_string(),
                    model: None,
                    tool: Some(Tool::McpTester),
                    main_prompt: None,
                    additional_prompts: vec![],
                },
            );
    }

    let mut pipelines: HashMap<String, PipelineConfig> = HashMap::new();
    for (pipeline_name, order) in pipeline_order {
        let stages = pipeline_stages.remove(&pipeline_name).unwrap_or_default();
        pipelines.insert(
            pipeline_name,
            PipelineConfig {
                order,
                stages,
                ..Default::default()
            },
        );
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
        Some("go_second".to_string()),
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
// Test 3: Calling a sub-pipeline via call_* MCP tool
// ===========================================================================

pub async fn run_call_mode(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_call_mode").await;
    let task_id = env
        .create_task("Call mode test", "Call mode test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-callmode");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // Role for entry stage needs call_sub in its tool list
    let mut roles = HashMap::new();
    roles.insert(
        "role_main".to_string(),
        RoleDefinition {
            tools: vec![
                "report_success".to_string(),
                "call_sub".to_string(),
            ],
            prompt: None,
        },
    );

    let workflow = build_workflow_with_roles(
        vec![
            StageDef {
                name: "entry",
                role: "role_main",
                pipeline: "main",
            },
            StageDef {
                name: "handler",
                role: "role_sub",
                pipeline: "sub",
            },
        ],
        roles,
    );

    let scenarios = scenarios_map(vec![
        (
            "role_main",
            abstract_scenarios::call_pipeline_then_succeed_scenario("sub"),
        ),
        ("role_sub", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // Step 1: runs "entry" → calls call_sub MCP tool, then report_success → call_sub signal
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("call_sub".to_string()));
    assert_eq!(
        task.stack.len(),
        1,
        "Stack should have the return-to-entry entry"
    );
    assert_eq!(task.stack[0].pipeline, "main");
    assert_eq!(
        task.stack[0].signal, "go_entry",
        "Should return to same stage (entry) after sub-pipeline"
    );

    // Run to completion: sub/handler + return → re-run entry (without call this time)
    // We need different scenarios for the re-run: just report_success without calling sub
    let scenarios_rerun = scenarios_map(vec![
        ("role_main", abstract_scenarios::report_and_finish_scenario()),
        ("role_sub", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.run_to_completion(task_id, &workflow, &scenarios_rerun, 10)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Return from sub-pipeline should complete");
}

// ===========================================================================
// Test 4: Multi-stage pipeline with sub-pipeline call and continuation
// ===========================================================================

pub async fn run_return_from_mode(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_return").await;
    let task_id = env
        .create_task("Return test", "Return test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-return");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // step_one → step_two (calls aux via MCP tool) → step_three
    // After aux returns, step_two re-runs, succeeds without call → advances to step_three
    let mut roles = HashMap::new();
    roles.insert(
        "role_two".to_string(),
        RoleDefinition {
            tools: vec![
                "report_success".to_string(),
                "call_aux".to_string(),
            ],
            prompt: None,
        },
    );

    let workflow = build_workflow_with_roles(
        vec![
            StageDef {
                name: "step_one",
                role: "role_one",
                pipeline: "main",
            },
            StageDef {
                name: "step_two",
                role: "role_two",
                pipeline: "main",
            },
            StageDef {
                name: "step_three",
                role: "role_three",
                pipeline: "main",
            },
            StageDef {
                name: "aux_step",
                role: "role_aux",
                pipeline: "aux",
            },
        ],
        roles,
    );

    // First pass: role_two calls call_aux then report_success
    let scenarios_with_call = scenarios_map(vec![
        ("role_one", abstract_scenarios::report_and_finish_scenario()),
        (
            "role_two",
            abstract_scenarios::call_pipeline_then_succeed_scenario("aux"),
        ),
        ("role_three", abstract_scenarios::report_and_finish_scenario()),
        ("role_aux", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // Step 1: main/step_one → go_step_two
    env.run_pipeline(task_id, &workflow, &scenarios_with_call)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("go_step_two".to_string()));

    // Step 2: main/step_two → call_aux (via MCP tool + report_success)
    env.continue_pipeline(task_id, &workflow, &scenarios_with_call)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("call_aux".to_string()),
        "call_aux MCP tool + report_success should produce call_aux signal"
    );
    assert_eq!(task.stack.len(), 1, "Stack should have return entry");
    assert_eq!(task.stack[0].pipeline, "main");
    assert_eq!(
        task.stack[0].signal, "go_step_two",
        "Should return to step_two after aux"
    );

    // Step 3+: aux/aux_step → return → re-run step_two (without call) → step_three → DONE
    // After return from aux, step_two re-runs. This time no call_aux, just report_success → advance
    let scenarios_no_call = scenarios_map(vec![
        ("role_one", abstract_scenarios::report_and_finish_scenario()),
        ("role_two", abstract_scenarios::report_and_finish_scenario()),
        ("role_three", abstract_scenarios::report_and_finish_scenario()),
        ("role_aux", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.run_to_completion(task_id, &workflow, &scenarios_no_call, 10)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state, "DONE",
        "After aux return, step_two re-runs, advances to step_three, then DONE"
    );
    assert!(task.stack.is_empty(), "Stack should be empty after completion");
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
        ("role_work", abstract_scenarios::report_and_finish_scenario()),
        ("role_resolve", abstract_scenarios::report_and_finish_scenario()),
    ]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("call_merge".to_string()),
        "Diverged work branch should trigger call_merge"
    );
    assert_eq!(
        task.stack.len(),
        1,
        "Stack should have the caller stage pushed"
    );
    assert_eq!(task.stack[0].pipeline, "main");
    assert_eq!(
        task.stack[0].signal, "go_work",
        "Stack entry should have signal go_work (re-run interrupted stage)"
    );

    // Step 2: run the conflict handler (merging/resolve) → return
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("return".to_string()));
    assert_eq!(task.state, "merge_PENDING");

    // Step 3: process return → stack pop → go_work in main pipeline
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("go_work".to_string()),
        "After return from conflict, signal should be go_work (popped from stack)"
    );
    assert_eq!(task.state, "main_PENDING");
    assert!(
        task.stack.is_empty(),
        "Stack should be empty after pop"
    );
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
    assert_eq!(task.state, "DONE", "READY task should dispatch and complete");
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

    let mut pipelines = HashMap::new();
    pipelines.insert(
        "main".to_string(),
        PipelineConfig {
            order: vec!["check".to_string(), "finish".to_string()],
            stages: {
                let mut m = HashMap::new();
                m.insert(
                    "check".to_string(),
                    StageDefinition {
                        role: "role_check".to_string(),
                        model: None,
                        tool: Some(Tool::McpTester),
                        main_prompt: None,
                        additional_prompts: vec![],
                    },
                );
                m.insert(
                    "finish".to_string(),
                    StageDefinition {
                        role: "role_finish".to_string(),
                        model: None,
                        tool: Some(Tool::McpTester),
                        main_prompt: None,
                        additional_prompts: vec![],
                    },
                );
                m
            },
            max_retries: 3,
        },
    );
    let workflow = WorkflowConfig {
        prompts_dir: None,
        pipelines,
        roles: Default::default(),
    };

    // First run: failure → return_failure → root pipeline restarts from first stage
    let scenarios_reject = scenarios_map(vec![
        ("role_check", abstract_scenarios::report_failure_scenario()),
        ("role_finish", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.run_pipeline(task_id, &workflow, &scenarios_reject)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("return_failure".to_string()),
        "report_failure should produce return_failure signal"
    );

    // Process the return_failure: root pipeline failed → restart from first stage (check)
    env.continue_pipeline(task_id, &workflow, &scenarios_reject)
        .await;
    let task = env.get_task(task_id).await;
    // After return_failure at root: should restart from first stage
    assert_eq!(task.state, "main_PENDING");
    assert!(!task.pause, "Should not be paused (retries not exhausted)");

    // Now run with success → should advance check → finish → DONE
    let scenarios_accept = scenarios_map(vec![
        ("role_check", abstract_scenarios::report_success_scenario()),
        ("role_finish", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.run_to_completion(task_id, &workflow, &scenarios_accept, 10)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Pipeline should complete after success");
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
// Test 10: Automatic undefined worktree handler
// ===========================================================================

/// Scenario that sets routing params (simulating preparator work).
fn preparator_scenario(repo_path: &str) -> String {
    format!(
        r#"name: Preparator Scenario
description: Set routing params and report success
timeout: 60
stop_on_failure: true

steps:
- name: Configure worktree
  operation:
    type: tool_call
    tool: configure_worktree
    arguments:
      destination_repository: "{repo_path}"
      destination_branch: "main"
      work_branch_postfix: "test"
  assertions:
    - type: success

- name: Report success
  operation:
    type: tool_call
    tool: report_success
    arguments:
      message: "Params set"
  assertions:
    - type: success
"#,
        repo_path = repo_path,
    )
}

pub async fn run_auto_undefined(env: &IntegrationTestEnv) {
    if env.target_repo.is_some() {
        eprintln!(
            "[{}] Skipping run_auto_undefined: requires local repo",
            env.name()
        );
        return;
    }

    let repo_path = env.create_git_repo("repo_auto_undefined").await;
    // Create task WITHOUT identity fields — no routing params
    let task_id = env
        .create_task("Undefined test", "Undefined test description", "READY")
        .await;
    // DO NOT set branches — this triggers the "undefined" worktree problem

    let workflow = build_workflow(vec![
        StageDef {
            name: "working",
            role: "role_work",
            pipeline: "main",
        },
        StageDef {
            name: "preparing",
            role: "role_prep",
            pipeline: "init",
        },
    ]);

    // Create a custom dispatcher with on_undefined set
    let scenarios = scenarios_map(vec![
        ("role_work", abstract_scenarios::report_and_finish_scenario()),
        (
            "role_prep",
            preparator_scenario(&repo_path.to_string_lossy()),
        ),
    ]);

    // First call: should detect undefined identity, dispatch to preparing pipeline
    {
        let task = env.get_task(task_id).await;
        let idx = std::sync::atomic::AtomicU64::new(0);
        let scenarios_dir = env
            .base_path
            .join("scenarios")
            .join(format!("undefined_{}", idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        tokio::fs::create_dir_all(&scenarios_dir)
            .await
            .expect("failed to create scenarios directory");
        let mut scenario_paths = HashMap::new();
        for (role, yaml) in &scenarios {
            let path = scenarios_dir.join(format!("{role}.yml"));
            tokio::fs::write(&path, yaml)
                .await
                .expect("failed to write scenario file");
            scenario_paths.insert(role.clone(), path);
        }
        let mcp_tester_config = zbobr_executor_mcp_tester::ZbobrExecutorMcpTesterConfig {
            scenarios: scenario_paths,
            ..Default::default()
        };
        let zbobr = env.make_dispatcher(workflow.clone());
        zbobr_dispatcher::cli::process_task(&zbobr, &task, Some(&mcp_tester_config))
            .await
            .unwrap();
    }

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("call_init".to_string()),
        "Undefined identity should trigger call_init"
    );
    assert_eq!(task.stack.len(), 1, "Stack should have go_working");
    assert_eq!(task.stack[0].pipeline, "main");
    assert_eq!(task.stack[0].signal, "go_working");
    assert_eq!(task.worktree_retries, 1, "worktree_retries should be incremented");
}

// ===========================================================================
// Test 11: Worktree retry limit
// ===========================================================================

pub async fn run_retry_limit(env: &IntegrationTestEnv) {
    if env.target_repo.is_some() {
        eprintln!(
            "[{}] Skipping run_retry_limit: requires local repo",
            env.name()
        );
        return;
    }

    let repo_path = env.create_git_repo("repo_retry_limit").await;
    let work_branch = "zbobr_conflict-retry-limit";

    // Create diverging branches
    git_in(&repo_path, &["checkout", "-b", work_branch]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "work version\n",
        "Work change",
    )
    .await;
    git_in(&repo_path, &["checkout", "main"]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "main version\n",
        "Main change",
    )
    .await;

    let task_id = env
        .create_task("Retry limit test", "Retry limit test description", "READY")
        .await;
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", work_branch)
        .await;

    // Set worktree_retries to the max already (simulating prior retries)
    {
        let weak = env.zbobr.task_backend().get_task(task_id).await.unwrap();
        let mutable = weak.upgrade().await.unwrap();
        mutable
            .modify_task(Box::new(|mut task| {
                task.worktree_retries = 1; // above the default limit (0)
                task
            }))
            .await
            .unwrap();
    }

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
        ("role_work", abstract_scenarios::report_and_finish_scenario()),
        ("role_resolve", abstract_scenarios::report_and_finish_scenario()),
    ]);

    env.run_pipeline(task_id, &workflow, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(
        task.pause,
        "Task should be paused when retry limit is reached"
    );
    assert_eq!(
        task.state, "main_PENDING",
        "Task should be in PENDING state"
    );
}

// ===========================================================================
// Test 12: Sub-pipeline self-retry on failure
// ===========================================================================

pub async fn run_sub_pipeline_failure_retry(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_sub_fail").await;
    let task_id = env
        .create_task("Sub-fail test", "Sub-pipeline failure retry test", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-subfail");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // main/entry calls sub-pipeline; sub/handler fails via report_failure
    // sub pipeline has max_retries: 1 so it can retry once
    let mut roles = HashMap::new();
    roles.insert(
        "role_main".to_string(),
        RoleDefinition {
            tools: vec![
                "report_success".to_string(),
                "call_sub".to_string(),
            ],
            prompt: None,
        },
    );

    let mut pipelines = HashMap::new();
    pipelines.insert(
        "main".to_string(),
        PipelineConfig {
            order: vec!["entry".to_string()],
            stages: {
                let mut m = HashMap::new();
                m.insert(
                    "entry".to_string(),
                    StageDefinition {
                        role: "role_main".to_string(),
                        model: None,
                        tool: Some(Tool::McpTester),
                        main_prompt: None,
                        additional_prompts: vec![],
                    },
                );
                m
            },
            ..Default::default()
        },
    );
    pipelines.insert(
        "sub".to_string(),
        PipelineConfig {
            order: vec!["handler".to_string()],
            stages: {
                let mut m = HashMap::new();
                m.insert(
                    "handler".to_string(),
                    StageDefinition {
                        role: "role_sub".to_string(),
                        model: None,
                        tool: Some(Tool::McpTester),
                        main_prompt: None,
                        additional_prompts: vec![],
                    },
                );
                m
            },
            max_retries: 1,
        },
    );
    let workflow = WorkflowConfig {
        prompts_dir: None,
        pipelines,
        roles,
    };

    // Step 1: run entry → calls call_sub
    let scenarios_call = scenarios_map(vec![
        (
            "role_main",
            abstract_scenarios::call_pipeline_then_succeed_scenario("sub"),
        ),
        ("role_sub", abstract_scenarios::report_failure_scenario()),
    ]);
    env.run_pipeline(task_id, &workflow, &scenarios_call).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("call_sub".to_string()));
    assert_eq!(task.stack.len(), 1, "Stack should have return entry");

    // Step 2: run sub/handler → report_failure → return_failure signal
    env.continue_pipeline(task_id, &workflow, &scenarios_call)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("return_failure".to_string()),
        "sub/handler report_failure should produce return_failure"
    );

    // Step 3: process return_failure → sub-pipeline retries itself (not return to caller)
    env.continue_pipeline(task_id, &workflow, &scenarios_call)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state, "sub_PENDING",
        "Failed sub-pipeline should retry itself, not return to caller"
    );
    assert_eq!(
        task.stack.len(),
        1,
        "Stack should be preserved during sub-pipeline retry"
    );
    assert!(!task.pause, "Should not be paused yet (retries not exhausted)");

    // Step 4: retry sub/handler, this time with success → should return to caller
    let scenarios_success = scenarios_map(vec![
        ("role_main", abstract_scenarios::report_and_finish_scenario()),
        ("role_sub", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.run_to_completion(task_id, &workflow, &scenarios_success, 10)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state, "DONE",
        "After sub-pipeline retry succeeds, should return to caller and complete"
    );
    assert!(task.stack.is_empty(), "Stack should be empty after completion");
}

// ===========================================================================
// Test 13: Sub-pipeline failure pauses when retries exhausted
// ===========================================================================

pub async fn run_sub_pipeline_failure_pause(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_sub_pause").await;
    let task_id = env
        .create_task("Sub-pause test", "Sub-pipeline failure pause test", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-subpause");
    let dest_repo = env.dest_repo(&repo_path);
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // sub pipeline with max_retries: 0 → first failure should pause immediately
    let mut roles = HashMap::new();
    roles.insert(
        "role_main".to_string(),
        RoleDefinition {
            tools: vec![
                "report_success".to_string(),
                "call_sub".to_string(),
            ],
            prompt: None,
        },
    );

    let mut pipelines = HashMap::new();
    pipelines.insert(
        "main".to_string(),
        PipelineConfig {
            order: vec!["entry".to_string()],
            stages: {
                let mut m = HashMap::new();
                m.insert(
                    "entry".to_string(),
                    StageDefinition {
                        role: "role_main".to_string(),
                        model: None,
                        tool: Some(Tool::McpTester),
                        main_prompt: None,
                        additional_prompts: vec![],
                    },
                );
                m
            },
            ..Default::default()
        },
    );
    pipelines.insert(
        "sub".to_string(),
        PipelineConfig {
            order: vec!["handler".to_string()],
            stages: {
                let mut m = HashMap::new();
                m.insert(
                    "handler".to_string(),
                    StageDefinition {
                        role: "role_sub".to_string(),
                        model: None,
                        tool: Some(Tool::McpTester),
                        main_prompt: None,
                        additional_prompts: vec![],
                    },
                );
                m
            },
            max_retries: 0, // no retries allowed
        },
    );
    let workflow = WorkflowConfig {
        prompts_dir: None,
        pipelines,
        roles,
    };

    // Step 1: run entry → calls call_sub
    let scenarios = scenarios_map(vec![
        (
            "role_main",
            abstract_scenarios::call_pipeline_then_succeed_scenario("sub"),
        ),
        ("role_sub", abstract_scenarios::report_failure_scenario()),
    ]);
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("call_sub".to_string()));

    // Step 2: run sub/handler → report_failure
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("return_failure".to_string()));

    // Step 3: process return_failure → retries(1) > max_retries(0) → pause
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert!(
        task.pause,
        "Sub-pipeline should be paused when retries exhausted"
    );
}
