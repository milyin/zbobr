# Update labels_to_state() signature and implementation

**File:** `zbobr-task-backend-github/src/github.rs`
**Function:** `labels_to_state()` (lines ~269–307)

## Change signature

```rust
// OLD:
fn labels_to_state(labels: &[IssueLabel]) -> State

// NEW:
fn labels_to_state(labels: &[IssueLabel], pipeline: Option<&str>, stage: Option<&str>) -> State
```

## Change implementation

Remove the `pipeline_value` and `stage_value` extraction from labels. Use the passed-in `pipeline` and `stage` parameters instead:

```rust
fn labels_to_state(labels: &[IssueLabel], pipeline: Option<&str>, stage: Option<&str>) -> State {
    let mut state_value: Option<&str> = None;

    for label in labels {
        if let Some(v) = label.name.strip_prefix(STATE_PREFIX) {
            state_value = Some(v);
        }
        // pipeline: and stage: labels are no longer read
    }

    match state_value {
        None => State::Empty,
        Some(v) if v == STATE_LABEL_DONE => State::Done,
        Some(v) if v == STATE_LABEL_PAUSE => State::Pause,
        Some(v) if v == STATE_LABEL_READY => State::Ready,
        Some(v) if v == STATE_LABEL_PENDING => match pipeline {
            Some(p) => State::Pending(Pipeline::from(p)),
            None => State::Unknown(format!("{}{}", STATE_PREFIX, STATE_LABEL_PENDING)),
        },
        Some(v) if v == STATE_LABEL_RUNNING => match (pipeline, stage) {
            (Some(p), Some(s)) => State::Running(Pipeline::from(p), Stage::from(s)),
            (None, Some(s)) => State::Unknown(format!("state:running, missing pipeline, stage:{s}")),
            (Some(p), None) => State::Unknown(format!("state:running, pipeline:{p}, missing stage")),
            (None, None) => State::Unknown("state:running, missing pipeline and stage".to_string()),
        },
        Some(other) => State::Unknown(format!("{}{other}", STATE_PREFIX)),
    }
}
```

**Why:** Pipeline and stage no longer come from labels; they come from the PARAMETERS section of the issue body.