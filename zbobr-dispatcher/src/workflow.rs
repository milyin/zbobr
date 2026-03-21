use std::collections::HashMap;
use std::path::PathBuf;

use zbobr_api::config::{
    PipelineConfig, RoleDefinition, StageDefinition, WorkflowConfig,
};
use zbobr_api::Task;

// Re-export constants for convenience.
pub const MAIN_PIPELINE: &str = WorkflowConfig::MAIN_PIPELINE;
pub const INIT_PIPELINE: &str = WorkflowConfig::INIT_PIPELINE;
pub const MERGE_PIPELINE: &str = WorkflowConfig::MERGE_PIPELINE;

/// Workflow wraps a `WorkflowConfig` and exposes state machine logic as methods.
#[derive(Clone, Debug)]
pub struct Workflow {
    config: WorkflowConfig,
}

/// Action determined by the state machine for the next step.
pub enum StateAction<'a> {
    /// Execute this stage definition: (pipeline_name, stage_name, stage_def).
    RunStage(&'a str, &'a str, &'a StageDefinition),
    /// Task is completed.
    Done,
    /// Task is paused, waiting for user.
    Paused,
    /// Nothing to do (no signal, no pending action).
    Idle,
}

impl Default for Workflow {
    fn default() -> Self {
        let mut pipelines = HashMap::new();
        let dummy_stage = StageDefinition {
            role: Some("default".to_string()),
            ..Default::default()
        };
        for name in [MAIN_PIPELINE, INIT_PIPELINE, MERGE_PIPELINE] {
            pipelines.insert(
                name.to_string(),
                PipelineConfig {
                    order: vec!["default".to_string()],
                    stages: [(
                        "default".to_string(),
                        dummy_stage.clone(),
                    )]
                    .into(),
                    ..Default::default()
                },
            );
        }
        Self {
            config: WorkflowConfig {
                prompts_dir: None,
                pipelines,
                roles: HashMap::new(),
            },
        }
    }
}

impl Workflow {
    pub fn new(config: WorkflowConfig) -> anyhow::Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Create a `Workflow` without validating the config.
    /// Use in tests or when the config is known to be partial.
    pub fn from_config(config: WorkflowConfig) -> Self {
        Self { config }
    }

    // -- Config access --

    pub fn config(&self) -> &WorkflowConfig {
        &self.config
    }

    // -- Delegated getters --

    pub fn pipeline(&self, name: &str) -> Option<&PipelineConfig> {
        self.config.pipeline(name)
    }

    pub fn stage(&self, pipeline: &str, stage: &str) -> Option<&StageDefinition> {
        self.config.stage(pipeline, stage)
    }

    pub fn all_stages(&self) -> Vec<(&str, &str, &StageDefinition)> {
        self.config.all_stages()
    }

    pub fn default_pipeline(&self) -> &str {
        self.config.default_pipeline()
    }

    pub fn pipeline_names(&self) -> Vec<&str> {
        self.config.pipeline_names()
    }

    pub fn find_stage_by_role(&self, role: &str) -> Option<(&str, &str, &StageDefinition)> {
        self.config.find_stage_by_role(role)
    }

    pub fn role_definition(&self, role: &str) -> Option<&RoleDefinition> {
        self.config.role_definition(role)
    }

    pub fn start_stage_for_pipeline(&self, pipeline: &str) -> Option<(&str, &StageDefinition)> {
        self.config.start_stage_for_pipeline(pipeline)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.config.validate()
    }

    pub fn prompts_dir(&self) -> Option<&PathBuf> {
        self.config.prompts_dir.as_ref()
    }

    pub fn roles(&self) -> &HashMap<String, RoleDefinition> {
        &self.config.roles
    }

    pub fn pipelines(&self) -> &HashMap<String, PipelineConfig> {
        &self.config.pipelines
    }

    // -- State machine --

    /// Given a task's current state/signal/stack and the workflow configuration,
    /// determine the next action to take.
    pub fn resolve_next_action(&self, task: &Task) -> anyhow::Result<StateAction<'_>> {
        self.resolve_inner(task, 0)
    }

    fn resolve_inner(&self, task: &Task, depth: usize) -> anyhow::Result<StateAction<'_>> {
        // Guard against infinite recursion
        if depth > 20 {
            anyhow::bail!(
                "State machine recursion limit exceeded for task #{}",
                task.id
            );
        }

        let state = &task.state;

        // Empty or READY state: initialize from stack or default pipeline
        if state.is_empty() || state == "READY" {
            if task.stack.is_empty() {
                // Push default pipeline's start stage
                let default_pipeline = self.config.default_pipeline();
                let (stage_name, stage_def) = self
                    .config
                    .start_stage_for_pipeline(default_pipeline)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No start stage for default pipeline '{}'",
                            default_pipeline
                        )
                    })?;
                return Ok(StateAction::RunStage(
                    default_pipeline,
                    stage_name,
                    stage_def,
                ));
            }
            // Stack not empty: pop and continue (conceptually — caller handles state transitions)
            // For now, use signal to determine action if present
            if let Some(ref signal) = task.signal {
                return self.resolve_signal(task, signal);
            }
            return Ok(StateAction::Idle);
        }

        if state == "DONE" {
            return Ok(StateAction::Done);
        }

        if state == "PAUSE" {
            return Ok(StateAction::Paused);
        }

        // State is "{pipeline}_PENDING" — dispatch based on signal
        if let Some(pipeline) = state.strip_suffix("_PENDING") {
            if let Some(ref signal) = task.signal {
                return self.resolve_signal_in_pipeline(task, signal, pipeline);
            }
            return Ok(StateAction::Idle);
        }

        // State is "{pipeline}_{stage}" — currently running, nothing to do
        Ok(StateAction::Idle)
    }

    fn resolve_signal(&self, task: &Task, signal: &str) -> anyhow::Result<StateAction<'_>> {
        if signal.strip_prefix("go_").is_some()
            || signal.starts_with("call_")
            || signal == "return"
            || signal == "return_failure"
            || signal == "retry_current"
        {
            let pipeline = pipeline_from_state(&task.state)
                .unwrap_or_else(|| self.config.default_pipeline().to_string());
            return self.resolve_signal_in_pipeline(task, signal, &pipeline);
        }
        Ok(StateAction::Idle)
    }

    fn resolve_signal_in_pipeline(
        &self,
        task: &Task,
        signal: &str,
        pipeline: &str,
    ) -> anyhow::Result<StateAction<'_>> {
        if let Some(target_stage) = signal.strip_prefix("go_") {
            let (pipeline_key, pipeline_config) =
                self.config.pipelines.get_key_value(pipeline).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Signal '{}' references unknown pipeline '{}'",
                        signal,
                        pipeline
                    )
                })?;
            let (stage_key, stage_def) = pipeline_config
                .stages
                .get_key_value(target_stage)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Signal '{}' references unknown stage '{}' in pipeline '{}'",
                        signal,
                        target_stage,
                        pipeline
                    )
                })?;
            return Ok(StateAction::RunStage(
                pipeline_key.as_str(),
                stage_key.as_str(),
                stage_def,
            ));
        }

        if let Some(target_pipeline) = signal.strip_prefix("call_") {
            let (pipeline_key, pipeline_config) = self
                .config
                .pipelines
                .get_key_value(target_pipeline)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Signal '{}' references unknown pipeline '{}' (no start stage)",
                        signal,
                        target_pipeline
                    )
                })?;
            let (stage_key, start) = pipeline_config.start_stage().ok_or_else(|| {
                anyhow::anyhow!("Pipeline '{}' has no start stage", target_pipeline)
            })?;
            return Ok(StateAction::RunStage(
                pipeline_key.as_str(),
                stage_key,
                start,
            ));
        }

        if signal == "return" || signal == "return_failure" {
            return Ok(StateAction::Done);
        }

        if signal == "retry_current" {
            // Parse stage name from state "{pipeline}_{stage}" or "{pipeline}_PENDING"
            let state = &task.state;
            if let Some(suffix) = state.strip_prefix(&format!("{pipeline}_")) {
                if suffix != "PENDING" {
                    // State is "{pipeline}_{stage}" — re-run that stage
                    let (pipeline_key, pipeline_config) =
                        self.config.pipelines.get_key_value(pipeline).ok_or_else(|| {
                            anyhow::anyhow!(
                                "retry_current: unknown pipeline '{}'",
                                pipeline
                            )
                        })?;
                    if let Some((stage_key, stage_def)) = pipeline_config.stages.get_key_value(suffix) {
                        return Ok(StateAction::RunStage(
                            pipeline_key.as_str(),
                            stage_key.as_str(),
                            stage_def,
                        ));
                    }
                }
            }
            // Fallback: idle (can't determine which stage to retry)
            return Ok(StateAction::Idle);
        }

        Ok(StateAction::Idle)
    }
}

/// Extract pipeline name from a state string like "main_PENDING" or "main_working".
pub fn pipeline_from_state(state: &str) -> Option<String> {
    if state.is_empty() || state == "READY" || state == "DONE" || state == "PAUSE" {
        return None;
    }
    // "{pipeline}_PENDING" or "{pipeline}_{stage}"
    state.find('_').map(|pos| state[..pos].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbobr_api::config::{PipelineConfig, StageDefinition, WorkflowConfig};

    /// Helper: build a minimal valid WorkflowConfig with main/init/merge pipelines.
    fn base_workflow() -> WorkflowConfig {
        let role_stage = |role: &str| StageDefinition {
            role: Some(role.to_string()),
            ..Default::default()
        };
        let single_pipeline = |stage_name: &str, role: &str| PipelineConfig {
            order: vec![stage_name.to_string()],
            stages: [(stage_name.to_string(), role_stage(role))].into(),
            ..Default::default()
        };
        WorkflowConfig {
            pipelines: [
                ("main".into(), single_pipeline("working", "worker")),
                ("init".into(), single_pipeline("preparing", "preparator")),
                ("merge".into(), single_pipeline("merging", "merger")),
            ]
            .into(),
            ..Default::default()
        }
    }

    #[test]
    fn call_stage_valid() {
        let mut wf = base_workflow();
        // Add a "review" pipeline and a call stage in main that calls it
        wf.pipelines.insert(
            "review".into(),
            PipelineConfig {
                order: vec!["checking".into()],
                stages: [(
                    "checking".into(),
                    StageDefinition {
                        role: Some("reviewer".into()),
                        ..Default::default()
                    },
                )]
                .into(),
                ..Default::default()
            },
        );
        let main = wf.pipelines.get_mut("main").unwrap();
        main.order = vec!["working".into(), "call_review".into()];
        main.stages.insert(
            "call_review".into(),
            StageDefinition {
                call: Some("review".into()),
                ..Default::default()
            },
        );
        assert!(wf.validate().is_ok());
    }

    #[test]
    fn call_stage_unknown_target() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.order.push("do_call".into());
        main.stages.insert(
            "do_call".into(),
            StageDefinition {
                call: Some("nonexistent".into()),
                ..Default::default()
            },
        );
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("calls unknown pipeline"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn stage_both_role_and_call() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.order.push("bad".into());
        main.stages.insert(
            "bad".into(),
            StageDefinition {
                role: Some("worker".into()),
                call: Some("init".into()),
                ..Default::default()
            },
        );
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("both 'role' and 'call'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn stage_neither_role_nor_call() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.order.push("empty".into());
        main.stages.insert("empty".into(), StageDefinition::default());
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("neither 'role' nor 'call'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn call_stage_toml_round_trip() {
        let toml_str = r#"
[pipelines.main]
order = ["setup", "working"]
[pipelines.main.stages.setup]
call = "init"
[pipelines.main.stages.working]
role = "worker"

[pipelines.init]
order = ["preparing"]
[pipelines.init.stages.preparing]
role = "preparator"

[pipelines.merge]
order = ["merging"]
[pipelines.merge.stages.merging]
role = "merger"
"#;
        let wf: WorkflowConfig = toml::from_str(toml_str).unwrap();
        assert!(wf.validate().is_ok());

        let setup = wf.stage("main", "setup").unwrap();
        assert_eq!(setup.call_pipeline(), Some("init"));
        assert_eq!(setup.role_name(), None);
        assert!(setup.is_call());

        let working = wf.stage("main", "working").unwrap();
        assert_eq!(working.role_name(), Some("worker"));
        assert_eq!(working.call_pipeline(), None);
        assert!(!working.is_call());
    }

    #[test]
    fn resolve_next_action_call_stage_returns_run_stage() {
        let mut wf = base_workflow();
        wf.pipelines.insert(
            "sub".into(),
            PipelineConfig {
                order: vec!["s1".into()],
                stages: [(
                    "s1".into(),
                    StageDefinition {
                        role: Some("worker".into()),
                        ..Default::default()
                    },
                )]
                .into(),
                ..Default::default()
            },
        );
        let main = wf.pipelines.get_mut("main").unwrap();
        main.order = vec!["call_sub".into()];
        main.stages.clear();
        main.stages.insert(
            "call_sub".into(),
            StageDefinition {
                call: Some("sub".into()),
                ..Default::default()
            },
        );
        let workflow = Workflow::from_config(wf);

        // Fresh task → state machine should resolve to RunStage for the call stage
        let task = Task {
            id: 1,
            title: String::new(),
            description: String::new(),
            state: String::new(),
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            checklist: vec![],
            signal: None,
            stack: vec![],
            pause: false,
            confirm: false,
            worktree_retries: 0,
            pipeline_retries: HashMap::new(),
            pipeline_run_id: 0,
            etag: None,
        };
        let action = workflow.resolve_next_action(&task).unwrap();
        match action {
            StateAction::RunStage(pipeline, stage, def) => {
                assert_eq!(pipeline, "main");
                assert_eq!(stage, "call_sub");
                assert!(def.is_call());
                assert_eq!(def.call_pipeline(), Some("sub"));
            }
            other => panic!("expected RunStage, got {:?}", match other {
                StateAction::Done => "Done",
                StateAction::Paused => "Paused",
                StateAction::Idle => "Idle",
                _ => "RunStage",
            }),
        }
    }

    #[test]
    fn find_stage_by_role_skips_call_stages() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.order = vec!["call_init".into(), "working".into()];
        main.stages.insert(
            "call_init".into(),
            StageDefinition {
                call: Some("init".into()),
                ..Default::default()
            },
        );
        // "worker" role should still be found on the "working" stage
        assert!(wf.find_stage_by_role("worker").is_some());
        // No stage has role matching call target name
        assert!(wf.find_stage_by_role("init").is_none());
    }
}
