## What

In `zbobr-dispatcher/src/prompts.rs`, three `VAR_*` template variable name constants are currently defined by aliasing `PARAM_*` constants imported from `zbobr_api`:

```
pub const VAR_DESTINATION_REPOSITORY: &str = PARAM_DESTINATION_REPOSITORY;
pub const VAR_DESTINATION_BRANCH: &str = PARAM_DESTINATION_BRANCH;
pub const VAR_WORK_BRANCH: &str = PARAM_WORK_BRANCH;
```

After step 1 removes these from zbobr-api, this import will break.

## Why

The VAR_ names are prompt template variable names used by the dispatcher. They happen to match the current PARAM_ string values, but this is coincidental — the dispatcher should own its own template variable names independently of the github backend's parameter key strings.

## How to apply

- Replace the three aliased constants with explicit string literals:
  - `pub const VAR_DESTINATION_REPOSITORY: &str = "destination_repository";`
  - `pub const VAR_DESTINATION_BRANCH: &str = "destination_branch";`
  - `pub const VAR_WORK_BRANCH: &str = "work_branch";`
- Remove the `PARAM_DESTINATION_BRANCH`, `PARAM_DESTINATION_REPOSITORY`, `PARAM_WORK_BRANCH` imports from `zbobr_api` in this file
- The string values remain the same, so no runtime behavior changes