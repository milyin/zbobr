use zbobr_api::config::{StageDefinition, WorkflowConfig};
use zbobr_api::Task;

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

/// Given a task's current state/signal/stack and the workflow configuration,
/// determine the next action to take.
pub fn resolve_next_action<'a>(
    task: &Task,
    workflow: &'a WorkflowConfig,
) -> anyhow::Result<StateAction<'a>> {
    resolve_inner(task, workflow, 0)
}

fn resolve_inner<'a>(
    task: &Task,
    workflow: &'a WorkflowConfig,
    depth: usize,
) -> anyhow::Result<StateAction<'a>> {
    // Guard against infinite recursion
    if depth > 20 {
        anyhow::bail!("State machine recursion limit exceeded for task #{}", task.id);
    }

    let state = &task.state;

    // Empty or READY state: initialize from stack or default pipeline
    if state.is_empty() || state == "READY" {
        if task.stack.is_empty() {
            // Push default pipeline's start stage
            let default_pipeline = workflow.default_pipeline();
            let (stage_name, stage_def) = workflow
                .start_stage_for_pipeline(default_pipeline)
                .ok_or_else(|| anyhow::anyhow!("No start stage for default pipeline '{}'", default_pipeline))?;
            return Ok(StateAction::RunStage(default_pipeline, stage_name, stage_def));
        }
        // Stack not empty: pop and continue (conceptually — caller handles state transitions)
        // For now, use signal to determine action if present
        if let Some(ref signal) = task.signal {
            return resolve_signal(task, signal, workflow);
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
            return resolve_signal_in_pipeline(task, signal, pipeline, workflow);
        }
        return Ok(StateAction::Idle);
    }

    // State is "{pipeline}_{stage}" — currently running, nothing to do
    Ok(StateAction::Idle)
}

fn resolve_signal<'a>(
    task: &Task,
    signal: &str,
    workflow: &'a WorkflowConfig,
) -> anyhow::Result<StateAction<'a>> {
    if signal.strip_prefix("go_").is_some() {
        // Find the stage in any pipeline — look at what pipeline we're in from state
        let pipeline = pipeline_from_state(&task.state)
            .unwrap_or_else(|| workflow.default_pipeline().to_string());
        return resolve_signal_in_pipeline(task, signal, &pipeline, workflow);
    }
    if signal.starts_with("call_") || signal == "return" {
        let pipeline = pipeline_from_state(&task.state)
            .unwrap_or_else(|| workflow.default_pipeline().to_string());
        return resolve_signal_in_pipeline(task, signal, &pipeline, workflow);
    }
    Ok(StateAction::Idle)
}

fn resolve_signal_in_pipeline<'a>(
    _task: &Task,
    signal: &str,
    pipeline: &str,
    workflow: &'a WorkflowConfig,
) -> anyhow::Result<StateAction<'a>> {
    if let Some(target_stage) = signal.strip_prefix("go_") {
        let (pipeline_key, pipeline_config) = workflow.pipelines.get_key_value(pipeline).ok_or_else(|| {
            anyhow::anyhow!(
                "Signal '{}' references unknown pipeline '{}'",
                signal,
                pipeline
            )
        })?;
        let (stage_key, stage_def) = pipeline_config.stages.get_key_value(target_stage).ok_or_else(|| {
            anyhow::anyhow!(
                "Signal '{}' references unknown stage '{}' in pipeline '{}'",
                signal,
                target_stage,
                pipeline
            )
        })?;
        return Ok(StateAction::RunStage(pipeline_key.as_str(), stage_key.as_str(), stage_def));
    }

    if let Some(target_pipeline) = signal.strip_prefix("call_") {
        // Get pipeline key from workflow's HashMap for lifetime correctness
        let (pipeline_key, pipeline_config) = workflow.pipelines.get_key_value(target_pipeline).ok_or_else(|| {
            anyhow::anyhow!(
                "Signal '{}' references unknown pipeline '{}' (no start stage)",
                signal,
                target_pipeline
            )
        })?;
        let (stage_key, start) = pipeline_config.start_stage().ok_or_else(|| {
            anyhow::anyhow!(
                "Pipeline '{}' has no start stage",
                target_pipeline
            )
        })?;
        return Ok(StateAction::RunStage(pipeline_key.as_str(), stage_key, start));
    }

    if signal == "return" {
        // Return with empty stack → Done
        // Return with stack → caller handles pop + re-dispatch
        return Ok(StateAction::Done);
    }

    Ok(StateAction::Idle)
}

/// Extract pipeline name from a state string like "main_PENDING" or "main_working".
fn pipeline_from_state(state: &str) -> Option<String> {
    if state.is_empty() || state == "READY" || state == "DONE" || state == "PAUSE" {
        return None;
    }
    // "{pipeline}_PENDING" or "{pipeline}_{stage}"
    if let Some(pos) = state.find('_') {
        Some(state[..pos].to_string())
    } else {
        None
    }
}
