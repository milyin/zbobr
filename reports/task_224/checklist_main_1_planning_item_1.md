## What

In `zbobr-task-backend-fs/src/fs.rs`, the `TaskFile` struct currently has a `parameters: HashMap<String, String>` field that stores only `pr_url` (as `PARAM_PR_URL` key). All other task fields (`destination_repository`, `destination_branch`, `work_branch`) are already first-class fields. Promote `pr_url` to a first-class field too, and remove the `parameters` HashMap entirely.

## Why

The fs backend is for testing only and has no need for a named-parameter hashmap. Fields should be stored directly as typed YAML fields, consistent with how the other routing fields are already stored. No backward compatibility is required.

## How to apply

- Add `pr_url: Option<String>` as a first-class field in `TaskFile` (with `#[serde(default)]`)
- Remove the `parameters: HashMap<String, String>` field from `TaskFile`
- Update `to_task()`: read `pr_url` from `self.pr_url` directly (not from `self.parameters.get(PARAM_PR_URL)`)
- Update `from_task()`: set `pr_url: task.pr_url.clone()` directly (not via HashMap insertion)
- Remove the `use std::collections::HashMap` import if no longer needed
- Remove the `PARAM_PR_URL` import from `zbobr_api`
- The analog pattern is how `destination_repository`, `destination_branch`, `work_branch` are already handled in the same file — follow the exact same pattern