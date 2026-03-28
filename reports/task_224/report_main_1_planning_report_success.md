# Plan: Move PARAM_* constants to github backend

## Approach

The PARAM_* constants are internal key names used to encode/decode `Task` fields in GitHub issue body text. They belong exclusively to the github task backend and should never have been part of the public API.

## Key decisions

- **No backward compatibility**: fs backend is test-only, system not deployed
- **Analog**: In the fs backend, `destination_repository/branch/work_branch` are already first-class YAML fields — `pr_url` follows the same pattern
- **Dispatcher independence**: VAR_* constants in prompts.rs become self-contained string literals (same values, but no longer imported from api)

## Steps

1. Remove all PARAM_* constants from zbobr-api, define them locally in zbobr-task-backend-github
2. In zbobr-task-backend-fs: add `pr_url: Option<String>` as first-class TaskFile field, remove `parameters: HashMap<String, String>`
3. In zbobr-dispatcher/prompts.rs: replace PARAM_* aliases with inline string literals for VAR_* constants