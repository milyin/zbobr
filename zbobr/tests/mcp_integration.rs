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

/// Return `(subcommand, executor_flag_suffix)` for the given stage.
///
/// `subcommand` is the name of the nested task subcommand that runs a role
/// session (e.g. "prepare").
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

/// Construct the common command-line arguments used by tests.
///
/// Every `zbobr` invocation in this module needs the same set of configuration
/// flags (paths for the dispatcher, the cli tool, git user information, etc.).
/// This helper builds and returns that list so callers can append the stage-
/// specific pieces (`setup`, executor flags, task ID, …) without repeating
/// themselves.
fn make_zbobr_config_args(tasks_dir: &Path, workspaces_dir: &Path) -> Vec<String> {
    let mut args = Vec::new();
    let mut push = |flag: &str, val: &str| {
        args.push(flag.to_string());
        args.push(val.to_string());
    };

    push("--dispatcher-workspaces", &workspaces_dir.to_string_lossy());
    push("--tasks-fs-tasks-dir", &tasks_dir.to_string_lossy());
    push("--dispatcher-backend", "filesystem");
    push("--dispatcher-cli-tool", "mcp-tester");
    push("--dispatcher-agent-github-token", "dummy-not-used");
    push("--dispatcher-git-user-name", "test-bot");
    push("--dispatcher-git-user-email", "test@example.com");

    args
}

/// Execute the `zbobr` binary using the standard test configuration
/// flags plus a specific command and any additional arguments.
///
/// This centralises the binary lookup, environment setup and execution so
/// callers only need to provide the top‑level command (e.g. `"setup"` or
/// `"task"`) and whatever command‑specific flags follow it.  For role
/// sessions the caller will push the secondary subcommand name onto
/// `command_args`.
async fn run_zbobr(
    tmp_path: &Path,
    tasks_dir: &Path,
    workspaces_dir: &Path,
    command: &str,
    command_args: &[&str],
) {
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let mut args = make_zbobr_config_args(tasks_dir, workspaces_dir);
    args.push(command.to_string());
    // convert the slice of &str to owned Strings and extend the argument list
    args.extend(command_args.iter().map(|s| s.to_string()));

    let status = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(tmp_path)
        .env("RUST_LOG", &rust_log)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .expect("failed to run zbobr");

    assert!(
        status.success(),
        "zbobr {} failed with exit code {:?}",
        command,
        status.code(),
    );
}

/// Like `run_zbobr` but captures and returns stdout as a `String`.  Useful for
/// commands that print data (for example the task creation command which
/// reports the new ID).
async fn run_zbobr_capture(
    tmp_path: &Path,
    tasks_dir: &Path,
    workspaces_dir: &Path,
    command: &str,
    command_args: &[&str],
) -> String {
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let mut args = make_zbobr_config_args(tasks_dir, workspaces_dir);
    args.push(command.to_string());
    args.extend(command_args.iter().map(|s| s.to_string()));

    let output = tokio::process::Command::new(zbobr_bin)
        .args(&args)
        .current_dir(tmp_path)
        .env("RUST_LOG", &rust_log)
        .output()
        .await
        .expect("failed to run zbobr");

    assert!(
        output.status.success(),
        "zbobr {} failed with exit code {:?}",
        command,
        output.status.code(),
    );

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Run `zbobr setup` to initialise the tasks and workspaces directories.
async fn run_zbobr_setup(
    tmp_path: &std::path::Path,
    tasks_dir: &std::path::Path,
    workspaces_dir: &std::path::Path,
) {
    run_zbobr(tmp_path, tasks_dir, workspaces_dir, "setup", &[]).await;
}

/// Create the shared directory layout and task, writing the assert-false
/// sentinel scenario file.  Returns `None` when `mcp-tester` is not
/// installed so the caller can skip gracefully.
///
/// **Important:** this integration test uses *only* the command‑line
/// interface to create and manipulate tasks.  It must not instantiate or
/// call backend implementations directly so that the same test works with any
/// backend (filesystem, GitHub, etc.).  Keeping the test CLI‑only ensures it
/// exercises the public API which is what downstream users rely on.
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

    // Create the shared task via the CLI rather than touching a backend
    // directly.  This keeps the test backend‑agnostic, which is the whole
    // point of exercising the public interface.
    let task_id = {
        // leverage a helper that captures stdout so we can parse the numeric ID
        let output = run_zbobr_capture(
            &tmp_path,
            &tasks_dir,
            &workspaces_dir,
            "task",
            &[
                "create",
                "--title",
                "Dummy Task",
                "--description",
                "Dummy task description",
                "--stage",
                "preparation",
            ],
        )
        .await;

        // output should be like "Created task #123"; parse the number.
        let line = output.lines().next().unwrap_or_default();
        line.trim()
            .strip_prefix("Created task #")
            .and_then(|s| s.parse::<u64>().ok())
            .expect("failed to parse task id from zbobr output")
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
        (
            "preparation",
            if flag_suffix == "preparation" {
                &scenario_path
            } else {
                af
            },
        ),
        (
            "planning",
            if flag_suffix == "planning" {
                &scenario_path
            } else {
                af
            },
        ),
        (
            "working",
            if flag_suffix == "working" {
                &scenario_path
            } else {
                af
            },
        ),
        (
            "reviewing",
            if flag_suffix == "reviewing" {
                &scenario_path
            } else {
                af
            },
        ),
        (
            "merging",
            if flag_suffix == "merging" {
                &scenario_path
            } else {
                af
            },
        ),
    ];

    // build the command-specific arguments: executor slots followed
    // by the task id.  `run_zbobr` will take care of the common flags and
    // actual execution.
    let mut cmd_args = Vec::new();
    for (slot, path) in all_slots {
        cmd_args.push(format!("--executor-mcp-tester-{slot}"));
        cmd_args.push(path.to_string_lossy().to_string());
    }
    cmd_args.push(env.task_id.to_string());

    // the CLI now expects `zbobr task <subcommand> ...`.
    // convert the dynamically constructed `cmd_args` (owned strings) into
    // a temporary vector of string slices for the helper call. We can
    // build the vector in one go using an iterator and `map`.
    let full_args_vec: Vec<&str> = std::iter::once(command)
        .chain(cmd_args.iter().map(|s| s.as_str()))
        .collect();

    run_zbobr(
        &env.tmp_path,
        &env.tasks_dir,
        &env.workspaces_dir,
        "task",
        &full_args_vec,
    )
    .await;
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

    run_stage_test(
        &env,
        Stage::Preparation,
        preparator_comprehensive_scenario(),
    )
    .await;
    run_stage_test(&env, Stage::Planning, planner_comprehensive_scenario()).await;
}
