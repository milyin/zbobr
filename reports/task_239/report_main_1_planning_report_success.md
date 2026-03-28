# Plan: instance filtering and stage title

## Approach chosen

`instance: String` lives in `ZbobrDispatcherConfig` (single source of truth). It is threaded to the GitHub task backend by injecting it into `ZbobrTaskBackendGithubConfig` in `commands.rs` before backend construction — same pattern as `allowed_usernames`.

## Key design decisions

- **One config field, one place**: users set `instance` once in `[dispatcher]`. Backend receives it programmatically.
- **GitHub label `zbobr:<instance>`**: created in setup, used as a server-side filter in `list_tasks()`. Force cleanup deletes orphaned `zbobr:*` labels.
- **Stage title**: `instance:pipeline:run_id:**stage**`. Added `instance: String` to `StageInfo` and `MdStageTitle` with `serde(default, skip_serializing_if = "String::is_empty")` for backward-compatible parsing.
- **FS backend**: no filtering added (used primarily for testing); `instance` is only relevant for GitHub label-based routing.

## Analog

`allowed_usernames` in `ZbobrTaskBackendGithubConfig` and the state label management in `setup()`.

## Checklist items

1. Add `instance: String` to `ZbobrDispatcherConfig` + update init defaults
2. Add `instance` to GitHub backend config; inject from dispatcher in commands.rs
3. GitHub backend setup: create `zbobr:<instance>` label; `--force` deletes other `zbobr:*` labels
4. GitHub backend `list_tasks`: add `labels` filter for `zbobr:<instance>`
5. Add `instance` to `StageInfo` + update `MdStageTitle` parser/formatter
6. Populate `instance` in `StageInfo` construction sites in dispatcher