use std::path::{Path, PathBuf};

use tempfile::TempDir;
use zbobr_dispatcher::Stage;

mod mcp_tester_scenarios;
use mcp_tester_scenarios::{
    assert_false_scenario, planner_comprehensive_scenario, preparator_comprehensive_scenario,
};

/// All paths and shared state for one test run.
struct TestEnv {
    /// Keeps the temporary directory alive for the duration of the test.
    _tmp: TempDir,
    tmp_path: PathBuf,
    tasks_dir: PathBuf,
    scenarios_dir: PathBuf,
    workspaces_dir: PathBuf,
    /// Path of the pre-written assert-false sentinel scenario.
    assert_false_path: PathBuf,
    /// ID of the task created during setup (reused across stages).
    task_id: u64,
}

/// Return `(cli_subcommand, executor_flag_suffix)` for the given stage.
///
/// `executor_flag_suffix` is the part after `--executor-mcp-tester-` in the
/// CLI flag, matching the field names in `ZbobrExecutorMcpTesterConfig`.
fn stage_meta(stage: Stage) -> (&'static str, &'static str) {
    match stage {
        Stage::Preparation => ("prepare", "preparation"),
        Stage::Planning => ("plan", "planning"),
        Stage::Working => ("work", "working"),
        Stage::Reviewing => ("review", "reviewing"),
        Stage::Merging => ("merge", "merging"),
        other => panic!("stage_meta: unsupported stage {other:?}"),
    }
}

/// Run `zbobr setup` to initialise the tasks and workspaces directories.
async fn run_zbobr_setup(
    tmp_path: &std::path::Path,
    tasks_dir: &std::path::Path,
    workspaces_dir: &std::path::Path,
) {
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let status = tokio::process::Command::new(zbobr_bin)
        .args([
            "--dispatcher-workspaces",         &workspaces_dir.to_string_lossy(),
            "--tasks-fs-tasks-dir",            &tasks_dir.to_string_lossy(),
            "--dispatcher-backend",            "filesystem",
            "--dispatcher-cli-tool",           "mcp-tester",
            "--dispatcher-agent-github-token", "dummy-not-used",
            "--dispatcher-git-user-name",      "test-bot",
            "--dispatcher-git-user-email",     "test@example.com",
            "setup",
        ])
        .current_dir(tmp_path)
        .env("RUST_LOG", &rust_log)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .expect("failed to run zbobr setup");

    assert!(status.success(), "zbobr setup failed");
}

/// Create the shared directory layout and task, writing the assert-false
/// sentinel scenario file.  Returns `None` when `mcp-tester` is not
/// installed so the caller can skip gracefully.
async fn setup_test_env() -> Option<TestEnv> {
    let mcp_check = tokio::process::Command::new("mcp-tester")
        .arg("--version")
        .output()
        .await;
    if mcp_check.is_err() || !mcp_check.unwrap().status.success() {
        eprintln!("Skipping test: mcp-tester not installed (cargo install mcp-tester)");
        return None;
    }

    let tmp = TempDir::new().expect("failed to create temp dir");
    let tmp_path = tmp.path().to_path_buf();

    let tasks_dir = tmp_path.join("tasks");
    let scenarios_dir = tmp_path.join("scenarios");
    let workspaces_dir = tmp_path.join("workspaces");

    // Use zbobr setup command to create tasks and workspaces directories.
    run_zbobr_setup(&tmp_path, &tasks_dir, &workspaces_dir).await;

    // Create the scenarios directory (test-specific, not managed by zbobr).
    tokio::fs::create_dir_all(&scenarios_dir)
        .await
        .expect("failed to create scenarios directory");

    // Write the permanent sentinel scenario once.
    let assert_false_path = scenarios_dir.join("assert_false.yml");
    tokio::fs::write(&assert_false_path, assert_false_scenario())
        .await
        .expect("failed to write assert_false scenario");

    // Create the shared task using the filesystem backend.
    let task_id = {
        use std::collections::HashMap;

        use zbobr_dispatcher::{Stage, backend::TaskBackend};
        use zbobr_task_backend_fs::{FilesystemTaskBackend, ZbobrTaskBackendFsArgs};

        let backend = FilesystemTaskBackend::new(
            None,
            ZbobrTaskBackendFsArgs {
                tasks_dir: Some(tasks_dir.clone()),
            },
            &tasks_dir,
        )
        .expect("failed to create task backend");

        backend
            .create_task(
                "Dummy Task",
                "Dummy task description",
                Stage::Preparation,
                None,
                None,
                HashMap::new(),
            )
            .await
            .expect("failed to create task")
    };

    Some(TestEnv {
        _tmp: tmp,
        tmp_path,
        tasks_dir,
        scenarios_dir,
        workspaces_dir,
        assert_false_path,
        task_id,
    })
}

/// Run the zbobr CLI for the given stage using the provided scenario YAML.
///
/// The scenario is passed to the executor slot that corresponds to `stage`;
/// all other slots receive the assert-false sentinel so that any accidental
/// routing to a wrong stage causes an immediate test failure.
async fn run_stage_test(env: &TestEnv, stage: Stage, scenario: String) {
    let (command, flag_suffix) = stage_meta(stage);

    // Write the stage-specific scenario to a dedicated file.
    let scenario_path = env.scenarios_dir.join(format!("{command}.yml"));
    tokio::fs::write(&scenario_path, scenario)
        .await
        .expect("failed to write stage scenario");

    let af: &Path = &env.assert_false_path;

    // Map every executor slot: the active stage gets the real scenario; all
    // others get the assert-false sentinel.
    let all_slots: &[(&str, &Path)] = &[
        ("preparation", if flag_suffix == "preparation" { &scenario_path } else { af }),
        ("planning",    if flag_suffix == "planning"    { &scenario_path } else { af }),
        ("working",     if flag_suffix == "working"     { &scenario_path } else { af }),
        ("reviewing",   if flag_suffix == "reviewing"   { &scenario_path } else { af }),
        ("merging",     if flag_suffix == "merging"     { &scenario_path } else { af }),
    ];

    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");

    let mut args: Vec<String> = Vec::new();
    let mut push = |flag: &str, val: &str| {
        args.push(flag.to_string());
        args.push(val.to_string());
    };

    push("--dispatcher-workspaces",         &env.workspaces_dir.to_string_lossy());
    push("--tasks-fs-tasks-dir",            &env.tasks_dir.to_string_lossy());
    push("--dispatcher-backend",            "filesystem");
    push("--dispatcher-cli-tool",           "mcp-tester");
    push("--dispatcher-agent-github-token", "dummy-not-used");
    push("--dispatcher-git-user-name",      "test-bot");
    push("--dispatcher-git-user-email",     "test@example.com");

    for (slot, path) in all_slots {
        push(
            &format!("--executor-mcp-tester-{slot}"),
            &path.to_string_lossy(),
        );
    }

    args.push(command.to_string());
    args.push(env.task_id.to_string());

    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let status = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(&env.tmp_path)
        .env("RUST_LOG", &rust_log)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .expect("failed to run zbobr binary");

    assert!(
        status.success(),
        "zbobr {} failed with exit code {:?}",
        command,
        status.code(),
    );
}

/// Integration test covering the Preparation and Planning stages.
///
/// Both stages share a single task so that parameters written by the
/// preparator (destination_branch, work_branch) are readable by the planner.
/// The Planning scenario exercises all planner tools except `pull_work` (git
/// setup is deferred to a future iteration).
#[tokio::test]
async fn test_preparation_and_planning() {
    let Some(env) = setup_test_env().await else {
        return;
    };

    run_stage_test(&env, Stage::Preparation, preparator_comprehensive_scenario()).await;
    run_stage_test(&env, Stage::Planning, planner_comprehensive_scenario()).await;
}
