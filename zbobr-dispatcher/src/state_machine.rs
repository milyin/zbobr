use zbobr_api::config::{PipelineConfig, StageDefinition};
use zbobr_api::Task;

/// Action determined by the state machine for the next step.
pub enum StateAction<'a> {
    /// Execute this stage definition.
    RunStage(&'a StageDefinition),
    /// Task is completed.
    Done,
    /// Task is paused, waiting for user.
    Paused,
    /// Nothing to do (no signal, no pending action).
    Idle,
}

/// Given a task's current state/signal/stack and the pipeline configuration,
/// determine the next action to take.
pub fn resolve_next_action<'a>(
    task: &Task,
    pipeline: &'a PipelineConfig,
) -> anyhow::Result<StateAction<'a>> {
    resolve_inner(task, pipeline, 0)
}

fn resolve_inner<'a>(
    task: &Task,
    pipeline: &'a PipelineConfig,
    depth: usize,
) -> anyhow::Result<StateAction<'a>> {
    // Guard against infinite recursion
    if depth > 20 {
        anyhow::bail!("State machine recursion limit exceeded for task #{}", task.id);
    }

    let state = &task.state;

    // Empty or READY state: initialize from stack or default mode
    if state.is_empty() || state == "READY" {
        if task.stack.is_empty() {
            // Push default mode's start stage
            let default_mode = pipeline
                .default_mode()
                .ok_or_else(|| anyhow::anyhow!("No default mode in pipeline config"))?;
            let start = pipeline
                .start_stage_for_mode(default_mode)
                .ok_or_else(|| anyhow::anyhow!("No start stage for default mode '{}'", default_mode))?;
            return Ok(StateAction::RunStage(start));
        }
        // Stack not empty: pop and continue (conceptually — caller handles state transitions)
        // For now, use signal to determine action if present
        if let Some(ref signal) = task.signal {
            return resolve_signal(task, signal, pipeline);
        }
        return Ok(StateAction::Idle);
    }

    if state == "DONE" {
        return Ok(StateAction::Done);
    }

    if state == "PAUSE" {
        return Ok(StateAction::Paused);
    }

    // State is "{mode}_PENDING" — dispatch based on signal
    if let Some(mode) = state.strip_suffix("_PENDING") {
        if let Some(ref signal) = task.signal {
            return resolve_signal_in_mode(task, signal, mode, pipeline);
        }
        return Ok(StateAction::Idle);
    }

    // State is "{mode}_{stage}" — currently running, nothing to do
    Ok(StateAction::Idle)
}

fn resolve_signal<'a>(
    task: &Task,
    signal: &str,
    pipeline: &'a PipelineConfig,
) -> anyhow::Result<StateAction<'a>> {
    if signal.strip_prefix("go_").is_some() {
        // Find the stage in any mode — look at what mode we're in from state
        let mode = mode_from_state(&task.state)
            .or_else(|| pipeline.default_mode().map(|s| s.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Cannot determine mode for signal '{}'", signal))?;
        return resolve_signal_in_mode(task, signal, &mode, pipeline);
    }
    if signal.starts_with("call_") || signal == "return" {
        let mode = mode_from_state(&task.state)
            .or_else(|| pipeline.default_mode().map(|s| s.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Cannot determine mode for signal '{}'", signal))?;
        return resolve_signal_in_mode(task, signal, &mode, pipeline);
    }
    Ok(StateAction::Idle)
}

fn resolve_signal_in_mode<'a>(
    _task: &Task,
    signal: &str,
    mode: &str,
    pipeline: &'a PipelineConfig,
) -> anyhow::Result<StateAction<'a>> {
    if let Some(stage_name) = signal.strip_prefix("go_") {
        let stage_def = pipeline.stage_by_name(mode, stage_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Signal '{}' references unknown stage '{}' in mode '{}'",
                signal,
                stage_name,
                mode
            )
        })?;
        return Ok(StateAction::RunStage(stage_def));
    }

    if let Some(target_mode) = signal.strip_prefix("call_") {
        let start = pipeline.start_stage_for_mode(target_mode).ok_or_else(|| {
            anyhow::anyhow!(
                "Signal '{}' references unknown mode '{}' (no start stage)",
                signal,
                target_mode
            )
        })?;
        return Ok(StateAction::RunStage(start));
    }

    if signal == "return" {
        // Return with empty stack → Done
        // Return with stack → caller handles pop + re-dispatch
        return Ok(StateAction::Done);
    }

    Ok(StateAction::Idle)
}

/// Extract mode name from a state string like "main_PENDING" or "main_working".
fn mode_from_state(state: &str) -> Option<String> {
    if state.is_empty() || state == "READY" || state == "DONE" || state == "PAUSE" {
        return None;
    }
    // "{mode}_PENDING" or "{mode}_{stage}"
    if let Some(pos) = state.find('_') {
        Some(state[..pos].to_string())
    } else {
        None
    }
}
