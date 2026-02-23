use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use zbobr_dispatcher::Stage;

use super::env::IntegrationTestEnv;
use super::mcp_tester_scenarios::{
    assert_false_scenario,
    planner_comprehensive_scenario,
    preparator_comprehensive_scenario,
};

/// Scenario content for the preparation stage.
pub struct PreparationScenario(pub String);

/// Scenario content for the planning stage.
pub struct PlanningScenario(pub String);

/// Create a preparation scenario for the given repository path.
pub fn preparation_scenario(repo_path: &str) -> PreparationScenario {
    PreparationScenario(preparator_comprehensive_scenario(repo_path))
}

/// Create a planning scenario.
pub fn planning_scenario() -> PlanningScenario {
    PlanningScenario(planner_comprehensive_scenario())
}

// ---------------------------------------------------------------------------
// Stage metadata
// ---------------------------------------------------------------------------

/// Return `(subcommand, executor_flag_suffix)` for the given stage.
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

// ---------------------------------------------------------------------------
// Stage runner
// ---------------------------------------------------------------------------

/// Global counter used to give every `run_stage_test` invocation a unique
/// scratch directory, avoiding collisions when tests run in parallel.
static SCENARIO_COUNTER: AtomicU64 = AtomicU64::new(0);

impl IntegrationTestEnv {
    /// Run the zbobr CLI for the given stage using the provided scenario YAML.
    ///
    /// A unique scratch directory is created under `self.base_path/scenarios/`
    /// for each invocation; all unused stage slots receive an assert-false
    /// sentinel so accidental mis-routing causes an immediate test failure.
    pub async fn run_stage_test(&self, task_id: u64, stage: Stage, scenario: String) {
        let (command, flag_suffix) = stage_meta(stage);

        let idx = SCENARIO_COUNTER.fetch_add(1, Ordering::Relaxed);
        let scenarios_dir = self
            .base_path
            .join("scenarios")
            .join(format!("{flag_suffix}_{idx}"));
        tokio::fs::create_dir_all(&scenarios_dir)
            .await
            .expect("failed to create scenarios directory");

        let scenario_path = scenarios_dir.join(format!("{command}.yml"));
        tokio::fs::write(&scenario_path, scenario)
            .await
            .expect("failed to write stage scenario");

        let assert_false_content = assert_false_scenario();
        let af_path = scenarios_dir.join("assert_false.yml");
        tokio::fs::write(&af_path, assert_false_content.as_bytes())
            .await
            .expect("failed to write assert_false scenario");
        let af = &af_path;

        let all_slots: &[(&str, &PathBuf)] = &[
            ("preparation", if flag_suffix == "preparation" { &scenario_path } else { af }),
            ("planning",    if flag_suffix == "planning"    { &scenario_path } else { af }),
            ("working",     if flag_suffix == "working"     { &scenario_path } else { af }),
            ("reviewing",   if flag_suffix == "reviewing"   { &scenario_path } else { af }),
            ("merging",     if flag_suffix == "merging"     { &scenario_path } else { af }),
        ];

        let mut cmd_args = Vec::new();
        for (slot, path) in all_slots {
            cmd_args.push(format!("--executor-mcp-tester-{slot}"));
            cmd_args.push(path.to_string_lossy().to_string());
        }
        cmd_args.push(task_id.to_string());

        let full_args_vec: Vec<&str> = std::iter::once(command)
            .chain(cmd_args.iter().map(|s| s.as_str()))
            .collect();

        self.run_zbobr("task", &full_args_vec).await;
    }

    /// Run the preparation stage for `task_id`.
    pub async fn run_preparation(&self, scenario: PreparationScenario, task_id: u64) {
        self.run_stage_test(task_id, Stage::Preparation, scenario.0).await;
    }

    /// Run the planning stage for `task_id`.
    pub async fn run_planning(&self, scenario: PlanningScenario, task_id: u64) {
        self.run_stage_test(task_id, Stage::Planning, scenario.0).await;
    }

    /// After planning, verify that `PULL_WORK_RETURN_VALUE` is populated and that
    /// the expected branches exist in the cloned workspace repository.
    pub async fn verify_planning(&self, task_id: u64) {
        let output = self.show_task(task_id).await;

        let mut pull_work_return_value = None;
        for line in output.lines() {
            if let Some(idx) = line.find("PULL_WORK_RETURN_VALUE=") {
                let val = line[idx + "PULL_WORK_RETURN_VALUE=".len()..].trim();
                let val = val.trim_end_matches('\'');
                pull_work_return_value = Some(val.to_string());
                break;
            }
        }
        let pull_work_return_value =
            pull_work_return_value.expect("PULL_WORK_RETURN_VALUE not found in task output");

        let parsed: serde_json::Value = serde_json::from_str(&pull_work_return_value)
            .expect("Failed to parse PULL_WORK_RETURN_VALUE as JSON");
        let path_str = parsed
            .get("result")
            .and_then(|v| v.as_str())
            .expect("result field not found or not a string");

        let cloned_repo_path = std::path::PathBuf::from(path_str);

        assert!(cloned_repo_path.exists(), "Cloned repo path does not exist");
        assert!(
            cloned_repo_path.starts_with(&self.workspaces_dir),
            "Cloned repo path is not inside workspaces_dir"
        );
        assert!(
            cloned_repo_path.join(".git").exists(),
            "Cloned repo is not a git repository"
        );

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

        let expected_work_branch = format!("zbobr_fix-{task_id}-test");
        assert!(
            branches_str.contains(&expected_work_branch),
            "Work branch '{expected_work_branch}' not found in cloned repo"
        );

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
}

