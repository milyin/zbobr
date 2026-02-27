use std::sync::atomic::{AtomicU64, Ordering};

use zbobr_dispatcher::Stage;

use super::env::IntegrationTestEnv;

// ---------------------------------------------------------------------------
// Stage metadata
// ---------------------------------------------------------------------------

/// Return `(subcommand, executor_flag_suffix)` for the given stage.
pub fn stage_meta(stage: Stage) -> (&'static str, &'static str) {
    match stage {
        Stage::Preparing => ("prepare", "preparation"),
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

/// Global counter used to give every `run_stage` invocation a unique
/// scratch directory, avoiding collisions when tests run in parallel.
static SCENARIO_COUNTER: AtomicU64 = AtomicU64::new(0);

impl IntegrationTestEnv {
    /// Run the zbobr CLI for the given stage using the provided scenario YAML.
    ///
    /// A unique scratch directory is created under `self.base_path/scenarios/`
    /// for each invocation.  Only the slot corresponding to `stage` is
    /// configured; the mcp-tester executor will error if the dispatcher ever
    /// tries to invoke a role that wasn't explicitly given a scenario.  This
    /// simplifies the test harness by removing the old "assert false" sentinel
    /// YAML.
    pub async fn run_stage(&self, task_id: u64, stage: Stage, scenario: String) {
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

        // only pass the scenario for the current stage; other slots are omitted
        let mut cmd_args = Vec::new();
        cmd_args.push(format!("--executor-mcp-tester-{flag_suffix}"));
        cmd_args.push(scenario_path.to_string_lossy().to_string());
        cmd_args.push(task_id.to_string());

        let full_args_vec: Vec<&str> = std::iter::once(command)
            .chain(cmd_args.iter().map(|s| s.as_str()))
            .collect();

        self.run_zbobr("task", &full_args_vec).await;
    }
}
