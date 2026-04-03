# Test: inline_dispatcher_tables unit tests

**File:** `zbobr/src/init.rs` (in the `#[cfg(test)]` module)

## Tests to add

### 1. `inline_dispatcher_tables_converts_providers_to_inline`
Build a `toml_edit::DocumentMut` with `dispatcher.providers` containing one or more regular sub-tables (non-inline), call `inline_dispatcher_tables`, then serialize to string and assert:
- Providers appear as inline tables: `copilot = { executor = "copilot" }` (curly-brace syntax on a single line)
- The output does **not** contain a `[dispatcher.providers.copilot]` section header

### 2. `inline_dispatcher_tables_converts_tools_to_inline_array`
Build a `toml_edit::DocumentMut` with `dispatcher.tools` containing one key whose value is an `ArrayOfTables` (i.e., multiple `[[dispatcher.tools.developer]]` sections), call `inline_dispatcher_tables`, then serialize to string and assert:
- Tools appear as an inline array of inline tables: `developer = [{ provider = "claude", model = "..." }, ...]`
- The output does **not** contain a `[[dispatcher.tools.developer]]` section header

### 3. `inline_dispatcher_tables_noop_when_dispatcher_absent`
Call `inline_dispatcher_tables` on a doc that has no `[dispatcher]` key and assert no panic.

### 4. `default_config_toml_uses_inline_dispatcher_format` (integration)
Call `default_config_toml()`, serialize, apply `inline_dispatcher_tables`, convert to string, and assert:
- Contains `copilot = {` (inline provider)
- Contains `developer = [` (inline tool array)
- Does **not** contain `[[dispatcher.tools.`

## Rationale
`inline_dispatcher_tables` is a new function with non-trivial toml_edit manipulation. The analogous `inline_stage_tables` has no unit tests either, but since `inline_dispatcher_tables` handles both a provider table and an array-of-tables → inline array conversion, it warrants explicit testing to catch regressions in the TOML formatting logic.
