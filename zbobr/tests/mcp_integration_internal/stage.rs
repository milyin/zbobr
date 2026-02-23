use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use zbobr_dispatcher::Stage;

use super::env::IntegrationTestEnv;

// sentinel scenario helper --------------------------------------------------

/// A simple scenario that always fails when executed.  It's written to every
/// unused executor slot during stage tests so that any unexpected routing of a
/// stage command into the wrong slot triggers an immediate failure.  The body
/// is basically copied from the old `mcp_tester_scenarios` module.
fn assert_false_scenario() -> String {
    use zbobr_dispatcher::mcp::preparator_tools::GET_DESCRIPTION;

    format!(
        r#"name: Assert False - must not run
description: Sentinel scenario – always fails on execution
timeout: 30
stop_on_failure: true

steps:
  - name: This stage must not execute
    operation:
      type: tool_call
      tool: {GET_DESCRIPTION}
    assertions:
      - type: equals
        path: result
        value: \"ASSERT_FALSE: this scenario must never be reached\""#,
    )
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

    /// Run the preparation stage for `task_id` using raw scenario YAML.
    pub async fn run_preparation(&self, scenario: String, task_id: u64) {
        self.run_stage_test(task_id, Stage::Preparation, scenario).await;
    }

    /// Run the planning stage for `task_id` using raw scenario YAML.
    pub async fn run_planning(&self, scenario: String, task_id: u64) {
        self.run_stage_test(task_id, Stage::Planning, scenario).await;
    }
}


