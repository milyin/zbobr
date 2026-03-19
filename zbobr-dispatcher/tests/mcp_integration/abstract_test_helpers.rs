//! Abstract pipeline tests that use generic stage/mode/role names.
//!
//! These tests verify the pipeline machinery independently of specific
//! naming conventions like "planning", "working", etc.
#![allow(dead_code)]

use std::collections::HashMap;

use zbobr_api::config::{PipelineConfig, StageDefinition};
use zbobr_dispatcher::task::Tool;

use super::{abstract_scenarios, env::IntegrationTestEnv};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct StageDef {
    name: &'static str,
    role: &'static str,
    mode: &'static str,
    is_start: bool,
    transitions: Vec<(&'static str, &'static str)>,
}

fn build_pipeline(stages: Vec<StageDef>) -> PipelineConfig {
    PipelineConfig {
        stages: stages
            .into_iter()
            .map(|s| StageDefinition {
                name: s.name.to_string(),
                role: s.role.to_string(),
                model: None,
                tool: Some(Tool::McpTester),
                main_prompt: None,
                additional_prompts: vec![],
                transitions: s
                    .transitions
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                is_start: s.is_start,
                mode: s.mode.to_string(),
            })
            .collect(),
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
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![StageDef {
        name: "alpha",
        role: "alpha",
        mode: "main",
        is_start: true,
        transitions: vec![("default", "return")],
    }]);

    let scenarios = scenarios_map(vec![(
        "alpha",
        abstract_scenarios::all_mcp_tools_scenario(&repo_path.to_string_lossy()),
    )]);

    env.run_pipeline(task_id, &pipeline, &scenarios).await;
    env.run_to_completion(task_id, &pipeline, &scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Should complete as DONE");
}

// ===========================================================================
// Test 2: Transfer between stages within a mode (go_X)
// ===========================================================================

pub async fn run_stage_transfer(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_transfer").await;
    let task_id = env
        .create_task("Transfer test", "Transfer test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-transfer");
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![
        StageDef {
            name: "first",
            role: "role_a",
            mode: "main",
            is_start: true,
            transitions: vec![("default", "go_second")],
        },
        StageDef {
            name: "second",
            role: "role_b",
            mode: "main",
            is_start: false,
            transitions: vec![("default", "return")],
        },
    ]);

    let scenarios = scenarios_map(vec![
        ("role_a", abstract_scenarios::report_and_finish_scenario()),
        ("role_b", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // First call runs "first" stage → go_second
    env.run_pipeline(task_id, &pipeline, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("go_second".to_string()));
    assert_eq!(task.state, "main_PENDING");

    // Run to completion: second stage + return resolution
    env.run_to_completion(task_id, &pipeline, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Should complete after both stages");
}

// ===========================================================================
// Test 3: Calling a sub-mode (call_X)
// ===========================================================================

pub async fn run_call_mode(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_call_mode").await;
    let task_id = env
        .create_task("Call mode test", "Call mode test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-callmode");
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![
        StageDef {
            name: "entry",
            role: "role_main",
            mode: "main",
            is_start: true,
            transitions: vec![("default", "call_sub")],
        },
        StageDef {
            name: "handler",
            role: "role_sub",
            mode: "sub",
            is_start: true,
            transitions: vec![("default", "return")],
        },
    ]);

    let scenarios = scenarios_map(vec![
        ("role_main", abstract_scenarios::report_and_finish_scenario()),
        ("role_sub", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // Step 1: runs "entry" → call_sub
    env.run_pipeline(task_id, &pipeline, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("call_sub".to_string()));

    // Run to completion: sub/handler + return
    env.run_to_completion(task_id, &pipeline, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Return from sub-mode should complete");
}

// ===========================================================================
// Test 4: Return from mode back to caller (multi-step)
// ===========================================================================

pub async fn run_return_from_mode(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_return").await;
    let task_id = env
        .create_task("Return test", "Return test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-return");
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![
        StageDef {
            name: "step_one",
            role: "role_one",
            mode: "main",
            is_start: true,
            transitions: vec![("default", "go_step_two")],
        },
        StageDef {
            name: "step_two",
            role: "role_two",
            mode: "main",
            is_start: false,
            transitions: vec![("default", "call_aux")],
        },
        StageDef {
            name: "aux_step",
            role: "role_aux",
            mode: "aux",
            is_start: true,
            transitions: vec![("default", "return")],
        },
    ]);

    let scenarios = scenarios_map(vec![
        ("role_one", abstract_scenarios::report_and_finish_scenario()),
        ("role_two", abstract_scenarios::report_and_finish_scenario()),
        ("role_aux", abstract_scenarios::report_and_finish_scenario()),
    ]);

    // Step 1: main/step_one → go_step_two
    env.run_pipeline(task_id, &pipeline, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("go_step_two".to_string()));

    // Step 2: main/step_two → call_aux
    env.continue_pipeline(task_id, &pipeline, &scenarios).await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.signal, Some("call_aux".to_string()));

    // Run to completion: aux_step → return → Done
    env.run_to_completion(task_id, &pipeline, &scenarios, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Return from aux should complete");
}

// ===========================================================================
// Test 5: Automatic conflict mode activation
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
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![
        StageDef {
            name: "work",
            role: "role_work",
            mode: "main",
            is_start: true,
            transitions: vec![("default", "return")],
        },
        StageDef {
            name: "resolve",
            role: "role_resolve",
            mode: "merging",
            is_start: true,
            transitions: vec![("default", "return")],
        },
    ]);

    let scenarios = scenarios_map(vec![
        ("role_work", abstract_scenarios::report_and_finish_scenario()),
        ("role_resolve", abstract_scenarios::report_and_finish_scenario()),
    ]);

    env.run_pipeline(task_id, &pipeline, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("call_merging".to_string()),
        "Diverged work branch should trigger call_merging"
    );
    assert!(
        !task.stack.is_empty(),
        "Stack should have the caller stage pushed"
    );
}

// ===========================================================================
// Test 6: Going to PAUSE via report_error
// ===========================================================================

pub async fn run_pause_on_error(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_pause").await;
    let task_id = env
        .create_task("Pause test", "Pause test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-pause");
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![StageDef {
        name: "work",
        role: "role_err",
        mode: "main",
        is_start: true,
        transitions: vec![("default", "return")],
    }]);

    let scenarios = scenarios_map(vec![(
        "role_err",
        abstract_scenarios::report_error_scenario(),
    )]);

    env.run_pipeline(task_id, &pipeline, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.pause, "report_error should set pause flag");
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
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![StageDef {
        name: "start",
        role: "role_start",
        mode: "main",
        is_start: true,
        transitions: vec![("default", "return")],
    }]);

    let scenarios = scenarios_map(vec![(
        "role_start",
        abstract_scenarios::report_and_finish_scenario(),
    )]);

    // Start from READY, run stage, then resolve return → DONE
    env.run_pipeline(task_id, &pipeline, &scenarios).await;
    env.run_to_completion(task_id, &pipeline, &scenarios, 5)
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "READY task should dispatch and complete");
}

// ===========================================================================
// Test 8: Signal-based transitions (review_accept / review_reject)
// ===========================================================================

pub async fn run_signal_transitions(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_signals").await;
    let task_id = env
        .create_task("Signal test", "Signal test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-signals");
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![
        StageDef {
            name: "check",
            role: "role_check",
            mode: "main",
            is_start: true,
            transitions: vec![
                ("review_accept", "go_finish"),
                ("review_reject", "go_check"),
                ("default", "return"),
            ],
        },
        StageDef {
            name: "finish",
            role: "role_finish",
            mode: "main",
            is_start: false,
            transitions: vec![("default", "return")],
        },
    ]);

    // First run: reject → go_check (loop back)
    let scenarios_reject = scenarios_map(vec![
        ("role_check", abstract_scenarios::signal_reject_scenario()),
        ("role_finish", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.run_pipeline(task_id, &pipeline, &scenarios_reject)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("go_check".to_string()),
        "review_reject should route to go_check"
    );

    // Second run: accept → go_finish
    let scenarios_accept = scenarios_map(vec![
        ("role_check", abstract_scenarios::signal_accept_scenario()),
        ("role_finish", abstract_scenarios::report_and_finish_scenario()),
    ]);
    env.continue_pipeline(task_id, &pipeline, &scenarios_accept)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some("go_finish".to_string()),
        "review_accept should route to go_finish"
    );

    // Run to completion: finish → return → DONE
    env.run_to_completion(task_id, &pipeline, &scenarios_accept, 5)
        .await;
    let task = env.get_task(task_id).await;
    assert_eq!(task.state, "DONE", "Pipeline should complete after finish");
}

// ===========================================================================
// Test 9: PAUSE via ask_user
// ===========================================================================

pub async fn run_pause_on_ask_user(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_ask").await;
    let task_id = env
        .create_task("Ask test", "Ask test description", "READY")
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-ask");
    env.update_task_branches(
        task_id,
        &repo_path.to_string_lossy(),
        "main",
        &work_branch,
    )
    .await;

    let pipeline = build_pipeline(vec![StageDef {
        name: "work",
        role: "role_ask",
        mode: "main",
        is_start: true,
        transitions: vec![("default", "return")],
    }]);

    let scenarios = scenarios_map(vec![(
        "role_ask",
        abstract_scenarios::ask_user_scenario(),
    )]);

    env.run_pipeline(task_id, &pipeline, &scenarios).await;

    let task = env.get_task(task_id).await;
    assert!(task.pause, "ask_user should set pause flag");
}
