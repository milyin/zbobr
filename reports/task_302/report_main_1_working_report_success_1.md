## Review fixes for multi-config support

Fixed both blocking issues identified in ctx_rec_9.

### Fix 1: Resolve relative paths per-config-file before merging

**Problem**: All relative paths were resolved against the last config file's directory after merging, breaking shared configs with relative paths.

**Solution**: 
- Generated `resolve_paths(self, config_dir: &Path) -> Self` method on all `*Toml` structs via the `config_struct` macro (`zbobr-macros/src/lib.rs`). For nested fields it recurses; for `#[config(path)]` fields it calls `resolve_path`; other fields pass through.
- Added manual `resolve_paths` to `WorkflowToml` (`zbobr-api/src/config.rs`) for the `prompts_dir` path field.
- Updated `main.rs` to call `resolve_paths` on each parsed config with that file's parent directory before merging. Since `resolve_path` is a no-op for absolute paths, the subsequent `Config::build` resolution is harmless.

### Fix 2: Handle attached short-value forms in CLI hoisting

**Problem**: `-cshared.toml` (attached form) wasn't recognized by the hoisting logic because `arg.split('=').next()` gives `-cshared.toml`, not `-c`.

**Solution**: Extended the hoist detection in `parse_cli` (`zbobr-dispatcher/src/cli.rs`) to check for attached short-value forms: if an arg starts with `-` (not `--`), is longer than 2 chars, and the first 2 chars match a short flag that takes a value, treat it as an attached short-value form and hoist it.

### Tests added
- `resolve_config_location_default_when_empty` — default path behavior
- `resolve_config_location_multiple_paths` — multi-file with correct config_dir
- `resolve_config_location_missing_file_errors` — error on nonexistent path
- `config_file_arg_short_flag_registered` — verifies -c short alias exists

### Commits
1. `fb833857` — fix: resolve relative paths per-config-file before merging
2. `97a5942a` — fix: handle attached short-value forms (-cval) in CLI hoisting
3. `efde01cb` — test: add tests for resolve_config_location and -c short flag

All workspace tests pass (3 pre-existing failures in zbobr-task-backend-github due to CryptoProvider, unrelated).