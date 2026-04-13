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
    match target {
        Some(t) => SequentialSignal::Advance(t),
        None => match no_target {
            Signal::Return => SequentialSignal::Return,
            Signal::ReturnFailure => SequentialSignal::ReturnFailure,
            _ => unreachable!(),
        },
    }
}

/// Workflow wraps a `WorkflowConfig` and exposes state machine logic as methods.
#[derive(Clone, Debug, Default)]
pub struct Workflow {
    config: WorkflowConfig,
}

/// Action determined by the state machine for the next step.
pub enum StateAction<'a> {
    /// Execute this stage definition: (pipeline_name, stage_name, stage_def).
    RunStage(&'a Pipeline, &'a Stage, &'a StageDefinition),
    /// Task is completed.
    Done,
    /// Task is paused — waiting for external action (resume handled by pre-pass).
    Paused,
    /// Nothing to do (no signal, no pending action).
    Idle,
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

    pub fn on_start(&self) -> Pipeline {
        self.config.on_start.clone().unwrap_or("main".into())
    }

    pub fn on_merge(&self) -> Pipeline {
        self.config.on_merge.clone().unwrap_or("merge".into())
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
                } else if def.is_pause() {
                    tracing::info!(
                        "Task #{}: resolved → RunStage {}/{} (pause)",
                        task.id,
                        pipeline,
                        stage,
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
                    tracing::info!(
                        "Task #{}: PENDING in pipeline '{}' with no signal → starting from first stage",
                        task.id,
                        pipeline
                    );
                    let (pipeline_key, pipeline_config) = self
                        .pipeline_entry(pipeline)
                        .ok_or_else(|| anyhow::anyhow!("Unknown pipeline '{pipeline}'"))?;
                    let (stage_name, stage_def) =
                        pipeline_config.start_stage().ok_or_else(|| {
                            anyhow::anyhow!("Pipeline '{pipeline}' has no start stage")
                        })?;
                    Ok(StateAction::RunStage(pipeline_key, stage_name, stage_def))
                }
            }
            State::Running(_, _) | State::Unknown(_) => Ok(StateAction::Idle),
        }
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
                let (pipeline_key, pipeline_config) =
                    self.pipeline_entry(pipeline).ok_or_else(|| {
                        anyhow::anyhow!(
                            "Signal '{signal}' references unknown pipeline '{pipeline}'"
                        )
                    })?;
                let (stage_key, stage_def) = pipeline_config
                    .stages
                    .as_ref()
                    .and_then(|stages| {
                        stages
                            .get_key_value(target_stage.as_str())
                            .and_then(|(k, v)| v.as_option().map(|v| (k, v)))
                    })
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
            Signal::Return | Signal::ReturnSuccess | Signal::ReturnFailure => {
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
            stages: Some(
                IndexMap::from([(Stage::from(stage_name), role_stage(role))])
                    .into_iter()
                    .map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v)))
                    .collect(),
            ),
        };
        WorkflowConfig {
            pipelines: Some(
                indexmap::IndexMap::from([
                    ("main".into(), single_pipeline("working", "worker")),
                    ("merge".into(), single_pipeline("merging", "merger")),
                ])
                .into_iter()
                .map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v)))
                .collect(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn call_stage_valid() {
        let mut wf = base_workflow();
        // Add a "review" pipeline and a call stage in main that calls it
        wf.insert_pipeline(
            zbobr_api::Pipeline::from("review"),
            PipelineConfig {
                stages: Some(
                    IndexMap::from([(
                        "checking".into(),
                        StageDefinition {
                            role: Some("reviewer".into()).into(),
                            ..Default::default()
                        },
                    )])
                    .into_iter()
                    .map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v)))
                    .collect(),
                ),
            },
        );
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.insert_stage(
            zbobr_api::Stage::from("call_review"),
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
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.insert_stage(
            zbobr_api::Stage::from("do_call"),
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
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.insert_stage(
            zbobr_api::Stage::from("bad"),
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
    fn stage_neither_role_nor_call_nor_pause() {
        let mut wf = base_workflow();
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.insert_stage(zbobr_api::Stage::from("empty"), StageDefinition::default());
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("must have exactly one of"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pause_stage_valid() {
        let mut wf = base_workflow();
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.insert_stage(
            zbobr_api::Stage::from("wait"),
            StageDefinition {
                pause: true,
                ..Default::default()
            },
        );
        assert!(wf.validate().is_ok());
    }

    #[test]
    fn pause_stage_with_role_fails() {
        let mut wf = base_workflow();
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.insert_stage(
            zbobr_api::Stage::from("bad"),
            StageDefinition {
                role: Some("worker".into()).into(),
                pause: true,
                ..Default::default()
            },
        );
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("must have exactly one of"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pause_stage_with_call_fails() {
        let mut wf = base_workflow();
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.insert_stage(
            zbobr_api::Stage::from("bad"),
            StageDefinition {
                call: Some("merge".into()).into(),
                pause: true,
                ..Default::default()
            },
        );
        let err = wf.validate().unwrap_err();
        assert!(
            err.to_string().contains("must have exactly one of"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pause_stage_toml_round_trip() {
        let toml_str = r#"
[pipelines.main.stages.working]
role = "worker"
on_success = "wait"

[pipelines.main.stages.wait]
pause = true

[pipelines.merge.stages.merging]
role = "merger"
"#;
        let wf: WorkflowConfig = toml::from_str(toml_str).unwrap();
        assert!(wf.validate().is_ok());

        let wait = wf
            .stage(&Pipeline::from("main"), &Stage::from("wait"))
            .unwrap();
        assert!(wait.is_pause());
        assert!(wait.role().is_none());
        assert!(wait.call_pipeline().is_none());
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
        wf.insert_pipeline(
            zbobr_api::Pipeline::from("sub"),
            PipelineConfig {
                stages: Some(IndexMap::from([(
                    zbobr_api::Stage::from("s1"),
                    zbobr_utility::TomlOption::Value(StageDefinition {
                        role: Some("worker".into()).into(),
                        ..Default::default()
                    }),
                )])),
            },
        );
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.stages = Some(
            IndexMap::from([(
                "call_sub".into(),
                StageDefinition {
                    call: Some("sub".into()).into(),
                    ..Default::default()
                },
            )])
            .into_iter()
            .map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v)))
            .collect(),
        );
        let workflow = Workflow::from_config(wf);

        // Pending task (no signal) → state machine should resolve to RunStage for the call stage
        let task = Task {
            id: 1,
            title: String::new(),
            description: String::new(),
            state: State::Pending(Pipeline::Main),
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
            dead_context: String::new(),
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
        wf.insert_pipeline(
            zbobr_api::Pipeline::from("sub"),
            PipelineConfig {
                stages: Some(IndexMap::from([(
                    zbobr_api::Stage::from("s1"),
                    zbobr_utility::TomlOption::Value(StageDefinition {
                        role: Some("helper".into()).into(),
                        ..Default::default()
                    }),
                )])),
            },
        );
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        let working = main
            .remove_stage(&zbobr_api::Stage::from("working"))
            .unwrap();
        main.insert_stage(
            zbobr_api::Stage::from("call_sub"),
            StageDefinition {
                call: Some("sub".into()).into(),
                ..Default::default()
            },
        );
        main.stages_mut()
            .insert(zbobr_api::Stage::from("working"), working);
        // "worker" role should still be found on the "working" stage
        assert!(wf.find_stage_by_role("worker").is_some());
        // No stage has role matching call target name
        assert!(wf.find_stage_by_role("sub").is_none());
    }

    #[test]
    fn on_success_unknown_stage_fails_validation() {
        let mut wf = base_workflow();
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.stage_mut(&zbobr_api::Stage::from("working"))
            .unwrap()
            .on_success = Some(zbobr_api::StageTransition::stage("nonexistent")).into();
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
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.stage_mut(&zbobr_api::Stage::from("working"))
            .unwrap()
            .on_failure = Some(zbobr_api::StageTransition::stage("nonexistent")).into();
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
        let main = wf.pipeline_mut(&zbobr_api::Pipeline::Main).unwrap();
        main.stage_mut(&zbobr_api::Stage::from("working"))
            .unwrap()
            .on_success = Some(zbobr_api::StageTransition::stage("working")).into();
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

}
