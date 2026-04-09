use indexmap::IndexMap;
use zbobr_api::{
    Pipeline, Signal, Stage, State, Task,
    config::{
        PipelineConfig, Role, RoleDefinition, StageDefinition, StageTransition, WorkflowConfig,
    },
    config_tools::McpTool,
};
use zbobr_utility::TomlOption;

/// Signal produced by the sequential pipeline model after a stage completes.
pub(crate) enum SequentialSignal {
    /// `report_failure` → immediate return from pipeline.
    ReturnFailure,
    /// `report_success`/`report_intermediate`/`on_no_report` with a next stage → advance to it.
    Advance(String),
    /// `report_success`/`on_no_report` at the last stage → pipeline done, return.
    Return,
    /// Stage-configured pause → set pause flag, emit given signal on resume.
    PauseThenSignal(Signal),
}

/// Convert a stage transition config + default target into a [`SequentialSignal`].
///
/// `no_target` is the signal to emit when neither the transition nor `default_target`
/// provides a stage; must be `Signal::Return` or `Signal::ReturnFailure`.
fn apply_transition(
    transition: Option<&StageTransition>,
    default_target: Option<String>,
    no_target: Signal,
) -> SequentialSignal {
    let target = transition
        .and_then(|t| t.next.as_ref())
        .map(|n: &Stage| n.to_string())
        .or(default_target);
    if transition.is_some_and(|t| t.pause) {
        SequentialSignal::PauseThenSignal(target.as_deref().map_or(no_target, Signal::go))
    } else {
        match target {
            Some(t) => SequentialSignal::Advance(t),
            None => match no_target {
                Signal::Return => SequentialSignal::Return,
                Signal::ReturnFailure => SequentialSignal::ReturnFailure,
                _ => unreachable!(),
            },
        }
    }
}

/// Workflow wraps a `WorkflowConfig` and exposes state machine logic as methods.
#[derive(Clone, Debug)]
pub struct Workflow {
    config: WorkflowConfig,
}

/// Action determined by the state machine for the next step.
pub enum StateAction<'a> {
    /// Execute this stage definition: (pipeline_name, stage_name, stage_def).
    RunStage(&'a Pipeline, &'a Stage, &'a StageDefinition),
    /// Task is completed.
    Done,
    /// Task is paused, waiting for user.
    Paused,
    /// Nothing to do (no signal, no pending action).
    Idle,
}

impl Default for Workflow {
    fn default() -> Self {
        let mut pipelines = IndexMap::new();
        let dummy_stage = StageDefinition {
            role: Some("default".to_string().into()).into(),
            ..Default::default()
        };
        for name in [Pipeline::MAIN, Pipeline::MERGE] {
            pipelines.insert(
                Pipeline::from(name),
                PipelineConfig {
                    stages: IndexMap::from([(Stage::from("default"), dummy_stage.clone())]),
                },
            );
        }
        Self {
            config: WorkflowConfig {
                prompts: None,
                pipelines: Some(
                    pipelines
                        .into_iter()
                        .map(|(k, v)| (k, TomlOption::Value(v)))
                        .collect(),
                ),
                roles: None,
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

    pub fn pipeline(&self, name: &Pipeline) -> Option<&PipelineConfig> {
        self.config.pipeline(name)
    }

    pub fn stage(&self, pipeline: &Pipeline, stage: &Stage) -> Option<&StageDefinition> {
        self.config.stage(pipeline, stage)
    }

    pub fn all_stages(&self) -> Vec<(&Pipeline, &str, &StageDefinition)> {
        self.config.all_stages()
    }

    pub fn default_pipeline(&self) -> Pipeline {
        self.config.default_pipeline()
    }

    pub fn pipeline_names(&self) -> Vec<&Pipeline> {
        self.config.pipeline_names()
    }

    pub fn find_stage_by_role(&self, role: &str) -> Option<(&Pipeline, &str, &StageDefinition)> {
        self.config.find_stage_by_role(role)
    }

    pub fn role_definition(&self, role: &str) -> Option<&RoleDefinition> {
        self.config.role_definition(role)
    }

    pub fn start_stage_for_pipeline(
        &self,
        pipeline: &Pipeline,
    ) -> Option<(&Stage, &StageDefinition)> {
        self.config.start_stage_for_pipeline(pipeline)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.config.validate()
    }

    pub fn roles(&self) -> &Option<IndexMap<Role, TomlOption<RoleDefinition>>> {
        &self.config.roles
    }

    pub fn pipelines(&self) -> &Option<IndexMap<Pipeline, TomlOption<PipelineConfig>>> {
        &self.config.pipelines
    }

    fn pipeline_entry(&self, name: &Pipeline) -> Option<(&Pipeline, &PipelineConfig)> {
        self.config
            .pipelines
            .as_ref()?
            .get_key_value(name.as_str())
            .and_then(|(k, v)| v.as_option().map(|cfg| (k, cfg)))
    }

    // -- Sequential pipeline model --

    /// Compute the post-execution signal for the sequential pipeline model.
    pub(crate) fn sequential_signal(
        &self,
        pipeline_name: &Pipeline,
        stage: &Stage,
        stage_def: Option<&StageDefinition>,
        last_mapped_tool: Option<McpTool>,
    ) -> SequentialSignal {
        let next_stage = || {
            self.pipeline(pipeline_name)
                .and_then(|p| p.next_stage(stage))
                .map(|(n, _)| n.to_string())
        };
        match last_mapped_tool {
            Some(McpTool::ReportFailure) => apply_transition(
                stage_def.and_then(|s| s.on_failure()),
                None,
                Signal::ReturnFailure,
            ),
            Some(McpTool::ReportSuccess) => apply_transition(
                stage_def.and_then(|s| s.on_success()),
                next_stage(),
                Signal::Return,
            ),
            Some(McpTool::ReportIntermediate) => apply_transition(
                stage_def.and_then(|s| s.on_intermediate()),
                Some(stage.to_string()),
                Signal::Return,
            ),
            // No report tool called — use on_no_report if configured, else advance (same as on_success).
            _ => apply_transition(
                stage_def.and_then(|s| s.on_no_report()),
                next_stage(),
                Signal::Return,
            ),
        }
    }

    // -- State machine --

    /// Given a task's current state/signal/stack and the workflow configuration,
    /// determine the next action to take.
    pub fn resolve_next_action(&self, task: &Task) -> anyhow::Result<StateAction<'_>> {
        tracing::debug!(
            "Task #{}: resolving next action (state={:?}, signal={:?}, stack_depth={})",
            task.id,
            task.state,
            task.signal,
            task.stack.len()
        );
        let action = self.resolve_inner(task, 0)?;
        match &action {
            StateAction::RunStage(pipeline, stage, def) => {
                if let Some(call_target) = def.call_pipeline() {
                    tracing::info!(
                        "Task #{}: resolved → RunStage {}/{} (call → {})",
                        task.id,
                        pipeline,
                        stage,
                        call_target
                    );
                } else {
                    tracing::info!(
                        "Task #{}: resolved → RunStage {}/{} (role={:?})",
                        task.id,
                        pipeline,
                        stage,
                        def.role()
                    );
                }
            }
            StateAction::Done => tracing::debug!("Task #{}: resolved → Done", task.id),
            StateAction::Paused => tracing::debug!("Task #{}: resolved → Paused", task.id),
            StateAction::Idle => tracing::debug!("Task #{}: resolved → Idle", task.id),
        }
        Ok(action)
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
            State::Empty => Ok(StateAction::Idle),
            State::Ready => {
                if task.stack.is_empty() {
                    // Push default pipeline's start stage
                    let default_pipeline = self.config.default_pipeline();
                    tracing::info!(
                        "Task #{}: READY with empty stack → starting default pipeline '{}'",
                        task.id,
                        default_pipeline
                    );
                    let (pipeline_key, pipeline_config) = self
                        .pipeline_entry(&default_pipeline)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "No start stage for default pipeline '{}'",
                                default_pipeline
                            )
                        })?;
                    let (stage_name, stage_def) =
                        pipeline_config.start_stage().ok_or_else(|| {
                            anyhow::anyhow!(
                                "No start stage for default pipeline '{}'",
                                default_pipeline
                            )
                        })?;
                    Ok(StateAction::RunStage(pipeline_key, stage_name, stage_def))
                } else if let Some(ref signal) = task.signal {
                    tracing::info!(
                        "Task #{}: READY with stack (depth={}) and signal '{}' → resolving signal",
                        task.id,
                        task.stack.len(),
                        signal
                    );
                    self.resolve_signal(task, signal)
                } else {
                    tracing::debug!("Task #{}: READY with stack but no signal → Idle", task.id);
                    Ok(StateAction::Idle)
                }
            }
            State::Done => Ok(StateAction::Done),
            State::Pause => Ok(StateAction::Paused),
            State::Pending(pipeline) => {
                if let Some(ref signal) = task.signal {
                    tracing::info!(
                        "Task #{}: PENDING in pipeline '{}' with signal '{}' → resolving",
                        task.id,
                        pipeline,
                        signal
                    );
                    self.resolve_signal_in_pipeline(signal, pipeline)
                } else {
                    tracing::debug!(
                        "Task #{}: PENDING in pipeline '{}' but no signal → Idle",
                        task.id,
                        pipeline
                    );
                    Ok(StateAction::Idle)
                }
            }
            State::Running(_, _) | State::Unknown(_) => Ok(StateAction::Idle),
        }
    }

    fn resolve_signal(&self, task: &Task, signal: &Signal) -> anyhow::Result<StateAction<'_>> {
        let pipeline =
            pipeline_from_state(&task.state).unwrap_or_else(|| self.config.default_pipeline());
        tracing::info!(
            "Task #{}: resolve_signal '{}' in pipeline '{}'",
            task.id,
            signal,
            pipeline
        );
        self.resolve_signal_in_pipeline(signal, &pipeline)
    }

    fn resolve_signal_in_pipeline(
        &self,
        signal: &Signal,
        pipeline: &Pipeline,
    ) -> anyhow::Result<StateAction<'_>> {
        match signal {
            Signal::Go(target_stage) => {
                tracing::info!(
                    "Signal Go('{}') → looking up stage in pipeline '{}'",
                    target_stage,
                    pipeline
                );
                let (pipeline_key, pipeline_config) = self
                    .pipeline_entry(pipeline)
                    .ok_or_else(|| {
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
                Ok(StateAction::RunStage(pipeline_key, stage_key, stage_def))
            }
            Signal::Call(target_pipeline) => {
                tracing::info!(
                    "Signal Call('{}') → entering sub-pipeline from '{}'",
                    target_pipeline,
                    pipeline
                );
                let (pipeline_key, pipeline_config) = self
                    .pipeline_entry(target_pipeline)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Signal '{signal}' references unknown pipeline '{target_pipeline}' (no start stage)"
                        )
                    })?;
                let (stage_key, start) = pipeline_config.start_stage().ok_or_else(|| {
                    anyhow::anyhow!("Pipeline '{target_pipeline}' has no start stage")
                })?;
                Ok(StateAction::RunStage(pipeline_key, stage_key, start))
            }
            Signal::Return | Signal::ReturnFailure => {
                tracing::info!(
                    "Signal '{}' → Done (returning from pipeline '{}')",
                    signal,
                    pipeline
                );
                Ok(StateAction::Done)
            }
        }
    }
}

/// Extract pipeline name from a typed task state.
pub fn pipeline_from_state(state: &State) -> Option<Pipeline> {
    state.pipeline().cloned()
}

#[cfg(test)]
mod tests {
    use zbobr_api::{
        TaskContext,
        config::{PipelineConfig, StageDefinition, WorkflowConfig},
    };

    use super::*;

    /// Helper: build a minimal valid WorkflowConfig with main/init/merge pipelines.
    fn base_workflow() -> WorkflowConfig {
        let role_stage = |role: &str| StageDefinition {
            role: Some(role.to_string().into()).into(),
            ..Default::default()
        };
        let single_pipeline = |stage_name: &str, role: &str| PipelineConfig {
            stages: IndexMap::from([(Stage::from(stage_name), role_stage(role))]),
        };
        WorkflowConfig {
            pipelines: [
                ("main".into(), single_pipeline("working", "worker")),
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
                        role: Some("reviewer".into()).into(),
                        ..Default::default()
                    },
                )]),
            },
        );
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.insert(
            "call_review".into(),
            StageDefinition {
                call: Some("review".into()).into(),
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
                call: Some("nonexistent".into()).into(),
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
                role: Some("worker".into()).into(),
                call: Some("merge".into()).into(),
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
        main.stages
            .insert("empty".into(), StageDefinition::default());
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
call = "sub"
[pipelines.main.stages.working]
role = "worker"

[pipelines.sub.stages.s1]
role = "helper"

[pipelines.merge.stages.merging]
role = "merger"
"#;
        let wf: WorkflowConfig = toml::from_str(toml_str).unwrap();
        assert!(wf.validate().is_ok());

        let setup = wf
            .stage(&Pipeline::from("main"), &Stage::from("setup"))
            .unwrap();
        assert_eq!(setup.call_pipeline().map(|p| p.as_str()), Some("sub"));
        assert_eq!(setup.role(), None);
        assert!(setup.is_call());

        let working = wf
            .stage(&Pipeline::from("main"), &Stage::from("working"))
            .unwrap();
        assert_eq!(
            working.role().as_ref().map(|role| role.as_str()),
            Some("worker")
        );
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
                        role: Some("worker".into()).into(),
                        ..Default::default()
                    },
                )]),
            },
        );
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages = IndexMap::from([(
            "call_sub".into(),
            StageDefinition {
                call: Some("sub".into()).into(),
                ..Default::default()
            },
        )]);
        let workflow = Workflow::from_config(wf);

        // Ready task → state machine should resolve to RunStage for the call stage
        let task = Task {
            id: 1,
            title: String::new(),
            description: String::new(),
            state: State::Ready,
            work_branch: None,
            pr_url: None,
            context: TaskContext::default(),
            signal: None,
            stack: vec![],
            status: None,
            go_pause: false,
            confirm: false,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: zbobr_api::task::DEFAULT_MAX_STAGE_COUNT,
            closed: false,
            etag: None,
        };
        let action = workflow.resolve_next_action(&task).unwrap();
        match action {
            StateAction::RunStage(pipeline, stage, def) => {
                assert_eq!(pipeline, &Pipeline::Main);
                assert_eq!(stage.as_str(), "call_sub");
                assert!(def.is_call());
                assert_eq!(def.call_pipeline().map(|p| p.as_str()), Some("sub"));
            }
            other => panic!(
                "expected RunStage, got {:?}",
                match other {
                    StateAction::Done => "Done",
                    StateAction::Paused => "Paused",
                    StateAction::Idle => "Idle",
                    _ => "RunStage",
                }
            ),
        }
    }

    #[test]
    fn find_stage_by_role_skips_call_stages() {
        let mut wf = base_workflow();
        wf.pipelines.insert(
            "sub".into(),
            PipelineConfig {
                stages: IndexMap::from([(
                    "s1".into(),
                    StageDefinition {
                        role: Some("helper".into()).into(),
                        ..Default::default()
                    },
                )]),
            },
        );
        let main = wf.pipelines.get_mut("main").unwrap();
        let working = main.stages.shift_remove("working").unwrap();
        main.stages.insert(
            "call_sub".into(),
            StageDefinition {
                call: Some("sub".into()).into(),
                ..Default::default()
            },
        );
        main.stages.insert("working".into(), working);
        // "worker" role should still be found on the "working" stage
        assert!(wf.find_stage_by_role("worker").is_some());
        // No stage has role matching call target name
        assert!(wf.find_stage_by_role("sub").is_none());
    }

    #[test]
    fn on_success_unknown_stage_fails_validation() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.get_mut("working").unwrap().on_success =
            Some(zbobr_api::StageTransition::stage("nonexistent")).into();
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("on_success references unknown stage"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn on_failure_unknown_stage_fails_validation() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.get_mut("working").unwrap().on_failure =
            Some(zbobr_api::StageTransition::stage("nonexistent")).into();
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("on_failure references unknown stage"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn on_success_self_reference_allowed() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.get_mut("working").unwrap().on_success =
            Some(zbobr_api::StageTransition::stage("working")).into();
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

[pipelines.merge.stages.merging]
role = "merger"
"#;
        let wf: WorkflowConfig = toml::from_str(toml_str).unwrap();
        assert!(wf.validate().is_ok());

        let working = wf
            .stage(&Pipeline::from("main"), &Stage::from("working"))
            .unwrap();
        assert_eq!(
            working
                .on_failure()
                .and_then(|t| t.next.as_ref())
                .map(|s| s.as_str()),
            Some("planning")
        );
        assert!(working.on_success().is_none());

        let planning = wf
            .stage(&Pipeline::from("main"), &Stage::from("planning"))
            .unwrap();
        assert_eq!(
            planning
                .on_success()
                .and_then(|t| t.next.as_ref())
                .map(|s| s.as_str()),
            Some("working")
        );
        assert!(planning.on_failure().is_none());
    }

    #[test]
    fn on_success_structured_with_pause_toml_round_trip() {
        let toml_str = r#"
[pipelines.main.stages.working]
role = "worker"
on_success = { pause = true }

[pipelines.main.stages.reviewing]
role = "reviewer"
on_success = { next = "working", pause = true }

[pipelines.merge.stages.merging]
role = "merger"
"#;
        let wf: WorkflowConfig = toml::from_str(toml_str).unwrap();
        assert!(wf.validate().is_ok());

        let working = wf
            .stage(&Pipeline::from("main"), &Stage::from("working"))
            .unwrap();
        let t = working.on_success().unwrap();
        assert!(t.next.is_none());
        assert!(t.pause);

        let reviewing = wf
            .stage(&Pipeline::from("main"), &Stage::from("reviewing"))
            .unwrap();
        let t = reviewing.on_success().unwrap();
        assert_eq!(t.next.as_ref().map(|s| s.as_str()), Some("working"));
        assert!(t.pause);
    }

    #[test]
    fn on_success_pause_only_passes_validation() {
        let mut wf = base_workflow();
        let main = wf.pipelines.get_mut("main").unwrap();
        main.stages.get_mut("working").unwrap().on_success =
            Some(zbobr_api::StageTransition::pause()).into();
        assert!(wf.validate().is_ok());
    }
}
