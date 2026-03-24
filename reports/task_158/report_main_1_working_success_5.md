# Checklist item: rewrite-state-serialization

## Changes made

File: `zbobr-api/src/task.rs`

### State Display format (new)
- `State::Empty` → `""` (unchanged)
- `State::Done` → `"state:done"`
- `State::Pause` → `"state:pause"`
- `State::Ready` → `"state:ready"`
- `State::Pending(pipeline)` → `"state:pending, pipeline:{name}"`
- `State::Running(pipeline, stage)` → `"state:running, pipeline:{name}, stage:{name}"`
- `State::Unknown(raw)` → raw verbatim (unchanged)

### State From&lt;&amp;str&gt; parser
- New format: splits on `", "`, matches `state:`/`pipeline:`/`stage:` prefixes
- Incomplete states (e.g. `"state:pending"` without pipeline, `"state:running, stage:bar"` without pipeline) correctly produce `State::Unknown`
- Legacy fallback: parses old milestone format (`"DONE"`, `"PAUSE"`, `"READY"`, `"main_PENDING"`, `"main_working"`) for backward compat with existing YAML files

### Removed
- Private constants: `State::DONE`, `State::PAUSE`, `State::READY`, `State::PENDING_SUFFIX`
- Private method: `State::equals_str()`

### Added
- `State::is_pending()` method
- `State::is_running()` method

### Simplified
- `PartialEq<&str>` and `PartialEq<String>` now compare via `to_string()` instead of custom `equals_str` logic

### Tests added (8 new tests)
- `state_display_new_format` - verifies all Display outputs
- `state_parse_new_format` - verifies parsing of new format
- `state_parse_new_format_incomplete_is_unknown` - verifies incomplete states
- `state_parse_legacy_format_fallback` - verifies old format still parses
- `state_roundtrip_display_parse` - verifies Display→From roundtrip
- `state_partial_eq_str_uses_new_format` - verifies PartialEq uses new format
- `state_is_pending` - verifies is_pending() method
- `state_is_running` - verifies is_running() method

### Verification
- All 22 zbobr-api tests pass
- Full project (`cargo check`) compiles clean with no warnings

### Note for next items
- The PartialEq<&str> now compares against the NEW format. Callers using `state == "DONE"` will no longer match; they need to use `state.is_done()` or compare against `"state:done"`. This is addressed by checklist items `update-callers` and `update-test-assertions`.
