**What:** Find and update all code that accesses `ConfigFileArg.path` or calls `resolve_config_location` with the old signature.

**Changes:**
- Update `GlobalArgs` field access patterns if any code reads `config_file.path` directly
- Update any other call sites of `resolve_config_location` to pass the new slice/vec type
- Ensure when no `--config` is passed, the default `zbobr.toml` behavior is preserved
- Ensure when one or more `--config` is passed, the default `zbobr.toml` is NOT loaded (as specified in requirements)

**Why:** The signature changes to ConfigFileArg and resolve_config_location will break existing callers. All must be updated to maintain compilation and correct behavior.

**Analog:** Search for all usages of `config_file.path` and `resolve_config_location` across the codebase to ensure nothing is missed.