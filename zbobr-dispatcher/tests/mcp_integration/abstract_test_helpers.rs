//! Abstract pipeline tests that use generic stage/mode/role names.
//!
//! These tests verify the pipeline machinery independently of specific
//! naming conventions like "planning", "working", etc.
#![allow(dead_code)]

use std::collections::HashMap;

use zbobr_api::config::{PipelineConfig, StageDefinition, WorkflowConfig};
use zbobr_dispatcher::task::Tool;

use super::{abstract_scenarios, env::IntegrationTestEnv};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct StageDef {
    name: &'static str,
    role: &'static str,
    pipeline: &'static str,
    is_start: bool,
    on_success: Option<&'static str>,
    on_failure: Option<&'static str>,
}

fn build_workflow(stages: Vec<StageDef>) -> WorkflowConfig {
    let mut pipelines: HashMap<String, PipelineConfig> = HashMap::new();
    for s in stages {
        let pipeline = pipelines
            .entry(s.pipeline.to_string())
            .or_insert_with(|| PipelineConfig {
                stages: HashMap::new(),
            });
        pipeline.stages.insert(
            s.name.to_string(),
            StageDefinition {
                role: s.role.to_string(),
                model: None,
                tool: Some(Tool::McpTester),
                main_prompt: None,
                additional_prompts: vec![],
                on_success: s.on_success.map(|v| v.to_string()),
                on_failure: s.on_failure.map(|v| v.to_string()),
                is_start: s.is_start,
            },
        );
    }
    WorkflowConfig {
        pipelines,
        roles: Default::default(),
    }
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
        is_start: true,
        on_success: None,
        on_failure: None,
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
// Test 2: Transfer between stages within a pipeline (go_X)
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
            is_start: true,
            on_success: Some("go_second"),
            on_failure: None,
        },
        StageDef {
            name: "second",
            role: "role_b",
            pipeline: "main",
            is_start: false,
            on_success: None,
            on_failure: None,
        },
    ]);

    let scenarios = scenarios_map(vec![
        ("role_a", abstract_scenarios::report_and_finish_scenario()),
        ("role_b", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // First call runs "first" stage → go_second
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("go_second".to_string()));
    assert_eq!(task.state, "main_PENDING");

    // Run to completion: second stage + return resolution
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Should complete after both stages");
}

// ===========================================================================
// Test 3: Calling a sub-pipeline (call_X)
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

    let workflow = build_workflow(vec![
        StageDef {
            name: "entry",
            role: "role_main",
            pipeline: "main",
            is_start: true,
            on_success: Some("call_sub"),
            on_failure: None,
        },
        StageDef {
            name: "handler",
            role: "role_sub",
            pipeline: "sub",
            is_start: true,
            on_success: None,
            on_failure: None,
        },
    ]);

    let scenarios = scenarios_map(vec![
        ("role_main", abstract_scenarios::report_and_finish_scenario()),
        ("role_sub", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // Step 1: runs "entry" → call_sub
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("call_sub".to_string()));

    // Run to completion: sub/handler + return
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Return from sub-pipeline should complete");
}

// ===========================================================================
// Test 4: Return from pipeline back to caller (multi-step)
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

    // Use compound call signal: "call_aux,go_done_step"
    // After returning from aux, the stack-aware Done handler pops go_done_step
    // and fires it in the main pipeline.
    let workflow = build_workflow(vec![
        StageDef {
            name: "step_one",
            role: "role_one",
            pipeline: "main",
            is_start: true,
            on_success: Some("go_step_two"),
            on_failure: None,
        },
        StageDef {
            name: "step_two",
            role: "role_two",
            pipeline: "main",
            is_start: false,
            on_success: Some("call_aux,go_done_step"),
            on_failure: None,
        },
        StageDef {
            name: "done_step",
            role: "role_done",
            pipeline: "main",
            is_start: false,
            on_success: None,
            on_failure: None,
        },
        StageDef {
            name: "aux_step",
            role: "role_aux",
            pipeline: "aux",
            is_start: true,
            on_success: None,
            on_failure: None,
        },
    ]);

    let scenarios = scenarios_map(vec![
        ("role_one", abstract_scenarios::report_and_finish_scenario()),
        ("role_two", abstract_scenarios::report_and_finish_scenario()),
        ("role_done", abstract_scenarios::report_and_finish_scenario()),
        ("role_aux", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // Step 1: main/step_one → go_step_two
    env.run_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("go_step_two".to_string()));

    // Step 2: main/step_two → call_aux (compound signal parsed)
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("call_aux".to_string()),
        "compound call should be split, signal is call_aux"
    );
    assert_eq!(
        task.stack.len(),
        1,
        "compound call should push after-return onto stack"
    );
    assert_eq!(task.stack[0].pipeline, "main");
    assert_eq!(task.stack[0].signal, "go_done_step");

    // Step 3: aux/aux_step → return → stack pop → go_done_step
    env.continue_pipeline(task_id, &workflow, &scenarios).await;
    // After return, state machine resolves Done, stack pops go_done_step
    // run_to_completion will handle the remaining transitions
    env.run_to_completion(task_id, &workflow, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.state, "DONE",
        "Return from aux should fire go_done_step, then done_step returns → DONE"
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
            is_start: true,
            on_success: None,
            on_failure: None,
        },
        StageDef {
            name: "resolve",
            role: "role_resolve",
            pipeline: "merging",
            is_start: true,
            on_success: None,
            on_failure: None,
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
        Some("call_merging".to_string()),
        "Diverged work branch should trigger call_merging"
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
    assert_eq!(task.state, "merging_PENDING");

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
        is_start: true,
        on_success: None,
        on_failure: None,
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
        is_start: true,
        on_success: None,
        on_failure: None,
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
// Test 8: Signal-based transitions (report_success / report_failure)
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

    let workflow = build_workflow(vec![
        StageDef {
            name: "check",
            role: "role_check",
            pipeline: "main",
            is_start: true,
            on_success: Some("go_finish"),
            on_failure: Some("go_check"),
        },
        StageDef {
            name: "finish",
            role: "role_finish",
            pipeline: "main",
            is_start: false,
            on_success: None,
            on_failure: None,
        },
    ]);

    // First run: failure → go_check (loop back)
    let scenarios_reject = scenarios_map(vec![
        ("role_check", abstract_scenarios::report_failure_scenario()),
        ("role_finish", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.run_pipeline(task_id, &workflow, &scenarios_reject)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("go_check".to_string()),
        "report_failure should route to go_check"
    );

    // Second run: success → go_finish
    let scenarios_accept = scenarios_map(vec![
        ("role_check", abstract_scenarios::report_success_scenario()),
        ("role_finish", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.continue_pipeline(task_id, &workflow, &scenarios_accept)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("go_finish".to_string()),
        "report_success should route to go_finish"
    );

    // Run to completion: finish → return → DONE
    env.run_to_completion(task_id, &workflow, &scenarios_accept, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Pipeline should complete after finish");
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
        is_start: true,
        on_success: None,
        on_failure: None,
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
            is_start: true,
            on_success: None,
            on_failure: None,
        },
        StageDef {
            name: "preparing",
            role: "role_prep",
            pipeline: "preparing",
            is_start: true,
            on_success: None,
            on_failure: None,
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

    // Override the dispatcher config to set on_undefined
    let config = zbobr_dispatcher::ZbobrDispatcherConfig {
        workspaces: env.workspaces_dir.clone(),
        tool: Tool::McpTester,
        on_conflict: Some("merging".to_string()),
        on_undefined: Some("preparing".to_string()),
        ..zbobr_dispatcher::ZbobrDispatcherConfig::default()
    };
    let zbobr_with_undefined = zbobr_dispatcher::ZbobrDispatcherBuilder::new()
        .with_config(std::sync::Arc::new(config))
        .with_task_backend(std::sync::Arc::clone(&env.task_backend))
        .with_repo_backend(std::sync::Arc::clone(&env.repo_backend))
        .with_prompt_builder(zbobr_dispatcher::prompts::ConfiguredPromptBuilder::new(
            None,
            std::sync::Arc::new(WorkflowConfig::default()),
        ))
        .build();

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
        let dispatcher = zbobr_with_undefined.with_mcp_tester_config(mcp_tester_config);
        zbobr_dispatcher::cli::process_task(&dispatcher, &task, &workflow)
            .await
            .unwrap();
    }

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("call_preparing".to_string()),
        "Undefined identity should trigger call_preparing"
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
        let weak = env.task_backend.get_task(task_id).await.unwrap();
        let mutable = weak.upgrade().await.unwrap();
        mutable
            .modify_task(Box::new(|mut task| {
                task.worktree_retries = 5; // at the limit (max_retries_conflict defaults to 5)
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
            is_start: true,
            on_success: None,
            on_failure: None,
        },
        StageDef {
            name: "resolve",
            role: "role_resolve",
            pipeline: "merging",
            is_start: true,
            on_success: None,
            on_failure: None,
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
