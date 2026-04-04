## Plan: Multi-Config Support via Multiple --config Parameters

### Rationale
The approach follows the existing `merge_with_args` pattern in the `config_struct` macro as the closest analog. Adding a `merge_toml` method mirrors this pattern for Toml×Toml merging, keeping the codebase consistent. The fold-based merge in main.rs is straightforward and matches the requirement that later configs override earlier ones.

### Key Design Decisions
- **Analog:** `merge_with_args` in `zbobr-macros/src/lib.rs` — new `merge_toml` follows the same recursive merge structure
- **Last file wins for config_dir:** Path resolution uses the last config file's parent directory, since that's the most project-specific config
- **Default skipped when explicit configs given:** When `--config`/`-c` is passed, `zbobr.toml` is not loaded — this is explicit in the requirements
- **Lists fully replace:** Since list fields are `Option<Vec<T>>`, the `other.or(self)` pattern naturally gives full replacement semantics
- **No config structure changes:** Per task requirement, only the loading/merging mechanism changes

### Checklist Summary
1. Change `ConfigFileArg` to `Vec<PathBuf>` with `-c` shortcut
2. Add `merge_toml` method generation to `config_struct` macro
3. Update `resolve_config_location` for multiple paths
4. Add fold-merge logic in `main.rs` config loading
5. Update all callers of changed interfaces