# Remove apply_signal_change() function and its call site

**File:** `zbobr-task-backend-github/src/github.rs`

## 1. Delete the apply_signal_change() function (lines ~425–464)

Remove the entire function — signal is now written to the issue body PARAMETERS section by `task_to_string_params()`, so no label management is needed.

## 2. Remove the call site in modify_task_internal() (lines ~985–987)

Remove these lines:
```rust
// REMOVE:
if task.signal != original_signal {
    self.apply_signal_change(id, task.signal.as_ref()).await?;
}
```

Also remove the `original_signal` variable declaration (line ~930):
```rust
// REMOVE:
let original_signal = task.signal.clone();
```

## 3. Remove signal_to_label() and label_to_signal() helper functions

Remove `signal_to_label()` (lines ~235–237) and `label_to_signal()` (lines ~240–242) as they are no longer used.

## 4. Remove SIGNAL_PREFIX constant (line ~21)

Remove:
```rust
const SIGNAL_PREFIX: &str = "signal:";
```

**Why:** Signal is now stored as a parameter string (e.g. `signal: go_planning`), not as a GitHub label.