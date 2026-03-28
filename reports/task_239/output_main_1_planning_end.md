Plan submitted for review. Here's the summary:

## Proposed Implementation Plan

**1. Config** — Add required `instance: String` to `ZbobrDispatcherConfig` (CLI: `--instance`, TOML: `instance = "..."`)

**2. `TaskBackend` trait** — Update signatures:
- `list_tasks(&self, instance: &str)`
- `setup(&self, force: bool, instance: &str)`

**3. GitHub backend `list_tasks`** — Add `labels=zbobr:<instance>` as a GitHub API query param (alongside existing filters)

**4. GitHub backend `setup`** — Create `zbobr:<instance>` label; without `--force` leave other `zbobr:*` labels alone; with `--force` delete mismatched `zbobr:*` labels

**5. Stage title** — Add `instance: String` to `MdStageTitle`; new format `instance:pipeline:run_id:**stage**`; backward-compatible parsing (detect format by whether 2nd or 3rd colon-segment is numeric)

**6. Wire up** — Pass `instance` from dispatcher config into `list_tasks()`, `setup()`, and stage title construction

Key design decision: `instance` lives in `ZbobrDispatcherConfig` (core concept, not GitHub-specific) and is passed to the backend via trait method parameters — same pattern as `force` in `setup()`.