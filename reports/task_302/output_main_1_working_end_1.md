All done. Here's a summary of the fixes:

1. **Path resolution** — Added `resolve_paths()` to all `*Toml` structs (macro-generated + manual for `WorkflowToml`). Each config file's relative paths are now resolved against its own directory before merging, so shared configs can safely use relative paths.

2. **`-c` hoisting** — Extended `parse_cli` to recognize attached short-value forms like `-cshared.toml`, not just `-c shared.toml` and `-c=shared.toml`.

3. **Tests** — Added 4 tests covering `resolve_config_location` behavior and the `-c` short flag registration.