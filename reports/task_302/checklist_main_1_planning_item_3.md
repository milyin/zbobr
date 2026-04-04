**What:** Update the config loading flow in `zbobr/src/main.rs` to iterate over multiple config paths, parse each into `RootConfigToml`, fold-merge them using the new `merge_toml` method, then pass the result to `RootConfig::build()`.

**Changes:**
- When config paths are provided: iterate each path, parse each to `RootConfigToml`, fold them together using `merge_toml` (first file is base, subsequent files override)
- When no paths provided (empty vec): keep current behavior — check if default `zbobr.toml` exists and optionally parse it
- Pass the final merged `Option<RootConfigToml>` to `RootConfig::build()` as before
- Update the call to `resolve_config_location` to pass the new paths argument

**Why:** This is where the actual multi-config merge happens at runtime. The fold pattern ensures configs are applied in order of appearance with later ones overriding earlier ones, matching user expectations.

**Analog:** Follow the existing config loading pattern in main.rs (lines ~84-96). The structure stays the same — just add the iteration and fold step before the existing `build()` call.