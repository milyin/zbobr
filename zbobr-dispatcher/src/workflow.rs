use std::collections::HashMap;
use std::path::PathBuf;

use indexmap::IndexMap;

use zbobr_api::config::{
    PipelineConfig, RoleDefinition, StageDefinition, WorkflowConfig,
};
use zbobr_api::{Pipeline, Signal, Stage, State, Task};

// Re-export constants for convenience.
pub const MAIN_PIPELINE: &str = Pipeline::MAIN;
pub const INIT_PIPELINE: &str = Pipeline::INIT;
pub const MERGE_PIPELINE: &str = Pipeline::MERGE;

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
                Pipeline::from(name),
                PipelineConfig {
                    stages: IndexMap::from([(
                        Stage::from("default"),
                        dummy_stage.clone(),
                    )]),
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
        self
            .config
            .all_stages()
            .into_iter()
            .map(|(pipeline, stage, def)| (pipeline.as_str(), stage, def))
            .collect()
    }

    pub fn default_pipeline(&self) -> &str {
        MAIN_PIPELINE
    }

    pub fn pipeline_names(&self) -> Vec<&str> {
        self
            .config
            .pipeline_names()
            .into_iter()
            .map(|pipeline| pipeline.as_str())
            .collect()
    }

    pub fn find_stage_by_role(&self, role: &str) -> Option<(&str, &str, &StageDefinition)> {
        self
            .config
            .find_stage_by_role(role)
            .map(|(pipeline, stage, def)| (pipeline.as_str(), stage, def))
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

    pub fn pipelines(&self) -> &HashMap<Pipeline, PipelineConfig> {
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

        match &task.state {
            State::Empty | State::Ready => {
                if task.stack.is_empty() {
                    // Push default pipeline's start stage
                    let default_pipeline = self.config.default_pipeline();
                    let (pipeline_key, pipeline_config) = self
                        .config
                        .pipelines
                        .get_key_value(default_pipeline.as_str())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "No start stage for default pipeline '{}'",
                                default_pipeline
                            )
                        })?;
                    let (stage_name, stage_def) = pipeline_config
                        .start_stage()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "No start stage for default pipeline '{}'",
                                default_pipeline
                            )
                        })?;
                    Ok(StateAction::RunStage(
                        pipeline_key.as_str(),
                        stage_name,
                        stage_def,
                    ))
                } else if let Some(ref signal) = task.signal {
                    self.resolve_signal(task, signal)
                } else {
                    Ok(StateAction::Idle)
                }
            }
            State::Done => Ok(StateAction::Done),
            State::Pause => Ok(StateAction::Paused),
            State::Pending(pipeline) => {
                if let Some(ref signal) = task.signal {
                    self.resolve_signal_in_pipeline(signal, pipeline)
                } else {
                    Ok(StateAction::Idle)
                }
            }
            State::Running(_, _) | State::Unknown(_) => Ok(StateAction::Idle),
        }
    }

    fn resolve_signal(&self, task: &Task, signal: &Signal) -> anyhow::Result<StateAction<'_>> {
        let pipeline = pipeline_from_state(&task.state)
            .unwrap_or_else(|| self.config.default_pipeline());
        self.resolve_signal_in_pipeline(signal, &pipeline)
    }

    fn resolve_signal_in_pipeline(
        &self,
        signal: &Signal,
        pipeline: &Pipeline,
    ) -> anyhow::Result<StateAction<'_>> {
        match signal {
            Signal::Go(target_stage) => {
                let (pipeline_key, pipeline_config) =
                    self.config.pipelines.get_key_value(pipeline.as_str()).ok_or_else(|| {
                        anyhow::anyhow!(
                            "Signal '{signal}' references unknown pipeline '{pipeline}'"
                        )
                    })?;
                let (stage_key, stage_def) = pipeline_config
                    .stages
                    .get_key_value(target_stage.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Signal '{signal}' references unknown stage '{target_stage}' in pipeline '{pipeline}'"
                        )
                    })?;
                Ok(StateAction::RunStage(
                    pipeline_key.as_str(),
                    stage_key.as_str(),
                    stage_def,
                ))
            }
            Signal::Call(target_pipeline) => {
                let (pipeline_key, pipeline_config) = self
                    .config
                    .pipelines
                    .get_key_value(target_pipeline.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Signal '{signal}' references unknown pipeline '{target_pipeline}' (no start stage)"
                        )
                    })?;
                let (stage_key, start) = pipeline_config.start_stage().ok_or_else(|| {
                    anyhow::anyhow!("Pipeline '{target_pipeline}' has no start stage")
                })?;
                Ok(StateAction::RunStage(
                    pipeline_key.as_str(),
                    stage_key,
                    start,
                ))
            }
            Signal::Return | Signal::ReturnFailure => Ok(StateAction::Done),
        }
    }
}

/// Extract pipeline name from a typed task state.
pub fn pipeline_from_state(state: &State) -> Option<Pipeline> {
    state.pipeline().cloned()
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
            stages: IndexMap::from([(Stage::from(stage_name), role_stage(role))]),
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
                stages: IndexMap::from([(
                    "checking".into(),
                    StageDefinition {
                        role: Some("reviewer".into()),
                        ..Default::default()
                    },
                )]),
            },
        );
        let main = wf.pipelines.get_mut("main").unwrap();
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
[pipelines.main.stages.setup]
call = "init"
[pipelines.main.stages.working]
role = "worker"

[pipelines.init.stages.preparing]
role = "preparator"

[pipelines.merge.stages.merging]
role = "merger"
"#;
        let wf: WorkflowConfig = toml::from_str(toml_str).unwrap();
        assert!(wf.validate().is_ok());

        let setup = wf.stage("main", "setup").unwrap();
        assert_eq!(setup.call_pipeline().map(|p| p.as_str()), Some("init"));
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
                stages: IndexMap::from([(
                    "s1".into(),
                    StageDefinition {
                        role: Some("worker".into()),
                        ..Default::default()
                    },
                )]),
            },
        );
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages = IndexMap::from([(
            "call_sub".into(),
            StageDefinition {
                call: Some("sub".into()),
                ..Default::default()
            },
        )]);
        let workflow = Workflow::from_config(wf);

        // Fresh task → state machine should resolve to RunStage for the call stage
        let task = Task {
            id: 1,
            title: String::new(),
            description: String::new(),
            state: State::Empty,
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            checklist: vec![],
            signal: None,
            stack: vec![],
            pause: false,
            confirm: false,
            pipeline_run_id: 0,
            etag: None,
        };
        let action = workflow.resolve_next_action(&task).unwrap();
        match action {
            StateAction::RunStage(pipeline, stage, def) => {
                assert_eq!(pipeline, "main");
                assert_eq!(stage, "call_sub");
                assert!(def.is_call());
                assert_eq!(def.call_pipeline().map(|p| p.as_str()), Some("sub"));
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
        let working = main.stages.shift_remove("working").unwrap();
        main.stages.insert(
            "call_init".into(),
            StageDefinition {
                call: Some("init".into()),
                ..Default::default()
            },
        );
        main.stages.insert("working".into(), working);
        // "worker" role should still be found on the "working" stage
        assert!(wf.find_stage_by_role("worker").is_some());
        // No stage has role matching call target name
        assert!(wf.find_stage_by_role("init").is_none());
    }

    #[test]
    fn on_success_unknown_stage_fails_validation() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.get_mut("working").unwrap().on_success =
            Some(zbobr_api::Stage::new("nonexistent"));
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("on_success references unknown stage"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn on_failure_unknown_stage_fails_validation() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.get_mut("working").unwrap().on_failure =
            Some(zbobr_api::Stage::new("nonexistent"));
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("on_failure references unknown stage"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn on_success_self_reference_allowed() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.get_mut("working").unwrap().on_success =
            Some(zbobr_api::Stage::new("working"));
        assert!(wf.validate().is_ok());
    }

    #[test]
    fn on_success_on_failure_toml_round_trip() {
        let toml_str = r#"
[pipelines.main.stages.working]
role = "worker"
on_failure = "planning"

[pipelines.main.stages.planning]
role = "planner"
on_success = "working"

[pipelines.init.stages.preparing]
role = "preparator"

[pipelines.merge.stages.merging]
role = "merger"
"#;
        let wf: WorkflowConfig = toml::from_str(toml_str).unwrap();
        assert!(wf.validate().is_ok());

        let working = wf.stage("main", "working").unwrap();
        assert_eq!(working.on_failure().map(|s| s.as_str()), Some("planning"));
        assert!(working.on_success().is_none());

        let planning = wf.stage("main", "planning").unwrap();
        assert_eq!(planning.on_success().map(|s| s.as_str()), Some("working"));
        assert!(planning.on_failure().is_none());
    }
}
