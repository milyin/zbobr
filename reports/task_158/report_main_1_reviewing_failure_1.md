# Review Report for Task 158: Replace Milestones with Labels

## Summary
The implementation successfully replaces milestone-based state storage with label-based storage. The state conversion logic and state representation via labels match the requirements. However, there is a critical regression regarding the handling of dynamic `Pipeline` and `Stage` values, and a minor discrepancy in color selection.

## Findings

### 1. Critical: Missing Dynamic Label Creation (Regression)
**Severity**: High
**Location**: `zbobr-task-backend-github/src/github.rs` in `apply_state_change`

The previous implementation using milestones explicitly handled the creation of missing milestones:
```rust
async fn get_or_create_milestone(&self, title: &str) -> anyhow::Result<u64>
```

The new implementation uses `apply_state_change` which calls `add_labels`:
```rust
self.octocrab.issues(owner, repo).add_labels(id, &new_labels).await
```
The GitHub API (and thus Octocrab) requires labels to exist in the repository before they can be added to an issue. If a label does not exist, the API call fails.

Since `Pipeline` and `Stage` can contain arbitrary user-defined strings (e.g., `Pipeline::Custom("deploy")`, `Stage("integration_test")`), and the `setup` function only pre-creates a fixed set of labels (`pipeline:main`, `pipeline:merge`), `apply_state_change` will fail at runtime for any custom pipeline or stage that hasn't been manually created.

**Recommendation**:
Update `apply_state_change` to check if the required labels exist (using `list_labels` or by handling the error) and create them using `create_label` if they are missing, similar to how `get_or_create_milestone` worked.

### 2. Minor: Color Mismatch
**Severity**: Low
**Location**: `zbobr-task-backend-github/src/github.rs` in `state_label_color`

The requirement stated: "pending is gray".
The implementation uses:
```rust
"state:pending" => "d4c5f9",    // gray
```
Hex code `d4c5f9` is **periwinkle/lavender**, not gray.
For a standard gray, consider using `#ededed` (GitHub's default label gray) or `#d3d3d3`.

### 3. Verification of Requirements
- **State Representation**: Correct (`state:{status}`, `pipeline:{name}`, `stage:{name}`).
- **Conversion Rules**: Correctly implemented in `labels_to_state`.
  - `state:running`, `stage:bar` correctly results in `State::Unknown`.
  - `state:pending` + pipeline correctly results in `State::Pending`.
- **Milestone Removal**: All milestone-related code has been removed.

## Conclusion
The logic is sound, but the removal of "get or create" semantics for the storage mechanism (previously milestones, now labels) breaks support for dynamic/custom pipelines and stages. This needs to be addressed to prevent runtime errors.
