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
            role: "default".to_string(),
            ..Default::default()
        };
        for name in [MAIN_PIPELINE, INIT_PIPELINE, MERGE_PIPELINE] {
            pipelines.insert(
                name.to_string(),
                PipelineConfig {
                    start: Some("default".to_string()),
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
        if signal.strip_prefix("go_").is_some() {
            // Find the stage in any pipeline — look at what pipeline we're in from state
            let pipeline = pipeline_from_state(&task.state)
                .unwrap_or_else(|| self.config.default_pipeline().to_string());
            return self.resolve_signal_in_pipeline(task, signal, &pipeline);
        }
        if signal.starts_with("call_") || signal == "return" {
            let pipeline = pipeline_from_state(&task.state)
                .unwrap_or_else(|| self.config.default_pipeline().to_string());
            return self.resolve_signal_in_pipeline(task, signal, &pipeline);
        }
        Ok(StateAction::Idle)
    }

    fn resolve_signal_in_pipeline(
        &self,
        _task: &Task,
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
            // Get pipeline key from workflow's HashMap for lifetime correctness
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

        if signal == "return" {
            // Return with empty stack → Done
            // Return with stack → caller handles pop + re-dispatch
            return Ok(StateAction::Done);
        }

        Ok(StateAction::Idle)
    }
}

/// Extract pipeline name from a state string like "main_PENDING" or "main_working".
fn pipeline_from_state(state: &str) -> Option<String> {
    if state.is_empty() || state == "READY" || state == "DONE" || state == "PAUSE" {
        return None;
    }
    // "{pipeline}_PENDING" or "{pipeline}_{stage}"
    state.find('_').map(|pos| state[..pos].to_string())
}
