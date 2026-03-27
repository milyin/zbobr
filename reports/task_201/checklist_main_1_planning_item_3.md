# Remove pipeline/stage labels from state_to_labels()

**File:** `zbobr-task-backend-github/src/github.rs`
**Function:** `state_to_labels()` (lines ~245–266)

## Change

Remove `pipeline_label` and `stage_label` closures and their usage. Return only the `state:*` label:

```rust
fn state_to_labels(state: &State) -> Vec<String> {
    let state_label = |name: &str| format!("{}{name}", STATE_PREFIX);

    match state {
        State::Empty => vec![],
        State::Done => vec![state_label(STATE_LABEL_DONE)],
        State::Pause => vec![state_label(STATE_LABEL_PAUSE)],
        State::Ready => vec![state_label(STATE_LABEL_READY)],
        State::Pending(_) => vec![state_label(STATE_LABEL_PENDING)],
        State::Running(_, _) => vec![state_label(STATE_LABEL_RUNNING)],
        State::Unknown(raw) => vec![state_label(raw)],
    }
}
```

**Why:** Pipeline and stage are now stored in PARAMETERS, not as labels. The state:* label remains so the issue has a visible status badge in GitHub.