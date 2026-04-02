# Tests for validate() global tool-name check — zbobr-api/src/config.rs

The latest commit added a check in `validate()`: when `self.tools` is non-empty, `self.tool` must be a key in `self.tools`. There are currently **zero tests** for this specific check.

## Tests to add (in existing `#[cfg(test)] mod tests` in config.rs)

### 1. `validate_rejects_unknown_global_tool`
- Setup: config with `tool = "nonexistent"`, `tools` map containing `"smart"` with a valid entry, and one valid provider
- Assert: `validate()` returns error containing "not defined in [tools]"

### 2. `validate_passes_when_tools_empty`
- Setup: config with `tool = "anything"`, `tools` map is empty (this is the "no tools configured" case — backward compat / simple setup)
- Assert: `validate()` succeeds (the guard `!self.tools.is_empty()` allows this)

### 3. `validate_passes_when_global_tool_exists`
- Setup: config with `tool = "smart"`, `tools` map containing `"smart"` with a valid entry, valid provider
- Assert: `validate()` succeeds
- This is essentially covered by `validate_valid_config` but naming it explicitly makes the intent clear

## Implementation notes
- Use existing `make_config()` helper.
- Tests 2 and 3 may overlap with existing `validate_valid_config` — if so, test 3 can be skipped in favor of just test 1 and test 2 (the boundary cases).
