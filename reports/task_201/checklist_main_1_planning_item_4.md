# Update apply_state_change() to not touch pipeline/stage labels

**File:** `zbobr-task-backend-github/src/github.rs`
**Function:** `apply_state_change()` (lines ~379–422)

## Change

In the label removal loop, remove the `pipeline:` and `stage:` prefix checks:

```rust
// OLD:
for label in &issue.labels {
    if label.name.starts_with(STATE_PREFIX)
        || label.name.starts_with(PIPELINE_PREFIX)
        || label.name.starts_with(STAGE_PREFIX)
    {

// NEW:
for label in &issue.labels {
    if label.name.starts_with(STATE_PREFIX) {
```

The rest of the function (adding new state labels) is unchanged since `state_to_labels()` now only returns state:* labels anyway.

**Note:** If there are lingering `pipeline:*` or `stage:*` labels on existing issues from before this change, they will remain. Since no backward compatibility effort is required, this is acceptable.