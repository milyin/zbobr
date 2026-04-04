Add a test to the existing `#[cfg(test)]` module in `zbobr-api/src/config.rs` that verifies `IndexMap<String, Vec<ToolEntry>>` merging behavior in the dispatcher config.

**What to test:**
- Base config defines tools "developer" and "reviewer" with different ToolEntry lists
- Overlay config overrides "developer" with a different list and adds "tester"
- After merge: "developer" entries are fully replaced by overlay (wholesale Vec replacement), "reviewer" survives from base unchanged, "tester" is added from overlay

This exercises the `config_struct` macro's generated `merge_toml` for map fields where the value type is `Vec<ToolEntry>` (which implements `MergeToml` as wholesale replacement). The macro generates key-wise map merge at `zbobr-macros/src/lib.rs` lines 322-338, and `Vec<T>::merge_toml` at `zbobr-utility/src/lib.rs` lines 24-28 simply returns the overlay.

**Why:** The tools map is the only `IndexMap<String, Vec<T>>` field — its merge behavior (key-wise with wholesale list replacement per key) is important for the config sharing use case where a project overlay might redefine tool entries for a specific tool name.

**Pattern to follow:** Same style as `dispatcher_toml_merge_providers_key_wise` test (line 1915). Use `ZbobrDispatcherConfigToml` structs with `Default::default()` for unrelated fields.