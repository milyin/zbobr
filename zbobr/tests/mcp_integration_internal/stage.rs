use std::path::{Path, PathBuf};

use zbobr_dispatcher::Stage;

use super::env::{TestEnv, run_zbobr, run_zbobr_capture};

use super::mcp_tester_scenarios::{
    planner_comprehensive_scenario,
    preparator_comprehensive_scenario,
};

/// Return `(subcommand, executor_flag_suffix)` for the given stage.
///
/// `subcommand` is the name of the nested task subcommand that runs a role
/// session (e.g. "prepare").
///
/// `executor_flag_suffix` is the part after `--executor-mcp-tester-` in the
/// CLI flag, matching the field names in `ZbobrExecutorMcpTesterConfig`.
pub fn stage_meta(stage: Stage) -> (&'static str, &'static str) {
    match stage {
        Stage::Preparation => ("prepare", "preparation"),
        Stage::Planning => ("plan", "planning"),
        Stage::Working => ("work", "working"),
        Stage::Reviewing => ("review", "reviewing"),
        Stage::Merging => ("merge", "merging"),
        other => panic!("stage_meta: unsupported stage {other:?}"),
    }
}

/// Run the zbobr CLI for the given stage using the provided scenario YAML.
///
/// The scenario is passed to the executor slot that corresponds to `stage`;
/// all other slots receive the assert-false sentinel so that any accidental
/// routing to a wrong stage causes an immediate test failure.
pub async fn run_stage_test(env: &TestEnv, stage: Stage, scenario: String) {
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
/// The Planning scenario exercises all planner tools including `pull_work`.
/// Create a minimal git repository used by the preparation stage.
/// Returns the path string that should be passed to the preparator scenario.
pub async fn create_test_repo(env: &TestEnv) -> String {
    let repo_dir = env.tmp_path.join("test_repo");
    tokio::fs::create_dir_all(&repo_dir).await.unwrap();

    // Initialize git repo and configure user
    let status = tokio::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .status()
        .await
        .unwrap();
    assert!(status.success());

    tokio::process::Command::new("git")
        .args(["config", "user.name", "test-bot"])
        .current_dir(&repo_dir)
        .status()
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_dir)
        .status()
        .await
        .unwrap();

    // initial commit and rename branch to main
    tokio::fs::write(repo_dir.join("README.md"), "test repo")
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["add", "README.md"])
        .current_dir(&repo_dir)
        .status()
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_dir)
        .status()
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(&repo_dir)
        .status()
        .await
        .unwrap();

    repo_dir.to_string_lossy().to_string()
}

/// Run the preparation stage against `repo_path`.
pub async fn test_preparation(env: &TestEnv, repo_path: &str) {
    run_stage_test(
        env,
        Stage::Preparation,
        preparator_comprehensive_scenario(repo_path),
    )
    .await;
}

/// Run the planning stage for the shared task.
pub async fn test_planning(env: &TestEnv) {
    run_stage_test(env, Stage::Planning, planner_comprehensive_scenario()).await;
}

/// After planning has run, examine the resulting task and verify that the
/// planner populated PULL_WORK_RETURN_VALUE correctly, turned it into a
/// working clone and set up branches as expected.
pub async fn verify_planning(env: &TestEnv) {
    // Read the task using zbobr task show and extract PULL_WORK_RETURN_VALUE
    let output = run_zbobr_capture(
        &env.tmp_path,
        &env.tasks_dir,
        &env.workspaces_dir,
        "task",
        &["show", &env.task_id.to_string()],
    )
    .await;

    // Find the line with PULL_WORK_RETURN_VALUE=xxxx
    let mut pull_work_return_value = None;
    for line in output.lines() {
        if let Some(idx) = line.find("PULL_WORK_RETURN_VALUE=") {
            let val = line[idx + "PULL_WORK_RETURN_VALUE=".len()..].trim();
            // The value might have a trailing quote from the YAML list item
            let val = val.trim_end_matches('\'');
            pull_work_return_value = Some(val.to_string());
            break;
        }
    }
    let pull_work_return_value =
        pull_work_return_value.expect("PULL_WORK_RETURN_VALUE not found in task output");

    // Parse the JSON to extract the actual path
    let parsed: serde_json::Value = serde_json::from_str(&pull_work_return_value)
        .expect("Failed to parse PULL_WORK_RETURN_VALUE as JSON");
    let path_str = parsed
        .get("result")
        .and_then(|v| v.as_str())
        .expect("result field not found or not a string");

    // Validate the path
    let cloned_repo_path = PathBuf::from(path_str);

    // this is a path and this path exists and it's inside the workspaces_dir
    assert!(cloned_repo_path.exists(), "Cloned repo path does not exist");
    assert!(
        cloned_repo_path.starts_with(&env.workspaces_dir),
        "Cloned repo path is not inside workspaces_dir"
    );

    // the path contains the git repository
    assert!(
        cloned_repo_path.join(".git").exists(),
        "Cloned repo is not a git repository"
    );

    // that there are branches named accordingly to PARAM_WORK_BRANCH and PARAM_DESTINATION_BRANCH in this repository
    // The preparator set destination branch to "main" and work branch postfix to "test"
    // The actual work branch name will be something like "zbobr_fix-1-test"
    let branches_output = tokio::process::Command::new("git")
        .arg("branch")
        .current_dir(&cloned_repo_path)
        .output()
        .await
        .unwrap();
    let branches_str = String::from_utf8_lossy(&branches_output.stdout);

    assert!(
        branches_str.contains("main"),
        "Destination branch 'main' not found in cloned repo"
    );

    let expected_work_branch = format!("zbobr_fix-{}-test", env.task_id);
    assert!(
        branches_str.contains(&expected_work_branch),
        "Work branch '{}' not found in cloned repo",
        expected_work_branch
    );

    // that the PARAM_WORK_BRANCH is current
    let current_branch_output = tokio::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&cloned_repo_path)
        .output()
        .await
        .unwrap();
    let current_branch = String::from_utf8_lossy(&current_branch_output.stdout)
        .trim()
        .to_string();
    assert_eq!(
        current_branch, expected_work_branch,
        "Current branch is not the work branch"
    );
}
