# Checklist item: rewrite-state-serde

## Changes made to `zbobr-api/src/task.rs`

### Removed
- `impl Display for State` — no longer has Display trait
- `impl PartialEq<&str> for State` — compared via Display
- `impl PartialEq<String> for State` — compared via Display
- `State::contains()` method — used Display internally
- `State::ends_with()` method — used Display internally
- Legacy milestone format parsing in `From<&str>` (DONE, PAUSE, READY, {pipeline}_PENDING, {pipeline}_{stage})

### Added
- `State::to_serde_string()` — private method for canonical serialization format
- Clean colon-separated format: `"done"`, `"pause"`, `"ready"`, `"pending:{pipeline}"`, `"running:{pipeline}:{stage}"`

### Updated
- `Serialize for State` — uses `to_serde_string()` instead of `to_string()`
- `From<&str> for State` — parses new colon-separated format only
- Tests rewritten to match new format
- Doc comment on `State` enum updated

### Expected downstream breakage (for items 3-4)
- `zbobr-task-backend-fs/src/fs.rs`: `task.state.to_string()` and `task.state == "DONE"`
- `zbobr-dispatcher/src/cli.rs`: `task.state.to_string()` and `{}`-formatting of State
- `zbobr-dispatcher/src/cleanup.rs`: `task.state == "DONE"`
- Test files: `task.state.contains()`, `task.state.ends_with()`

### Test results
All 20 zbobr-api tests pass.