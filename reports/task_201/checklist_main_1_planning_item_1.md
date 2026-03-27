# Read pipeline, stage, signal from PARAMETERS section in issue_to_task()

**File:** `zbobr-task-backend-github/src/github.rs`
**Function:** `issue_to_task()` (lines ~731–796)

## Changes

### 1. Read pipeline, stage, signal from params_map

Add after existing `params_map` extractions (after `pr_url`, before state):

```rust
let pipeline_param = params_map.get("pipeline").map(|s| s.as_str());
let stage_param = params_map.get("stage").map(|s| s.as_str());
let signal: Option<zbobr_api::Signal> = params_map
    .get("signal")
    .and_then(|s| s.parse().ok());
```

### 2. Change state reconstruction to use params instead of labels

Replace the existing state parsing line:
```rust
// OLD:
let state = Self::labels_to_state(&issue.labels);
```
with:
```rust
// NEW:
let state = Self::labels_to_state(&issue.labels, pipeline_param, stage_param);
```

### 3. Remove signal extraction from labels

Remove these lines (signal is now from params):
```rust
// REMOVE:
let signal = issue
    .labels
    .iter()
    .find_map(|l| Self::label_to_signal(&l.name));
```

**Why:** Pipeline, stage, and signal are now in the PARAMETERS section of the issue body. The `signal` variable is still set — just sourced from `params_map` instead of labels.