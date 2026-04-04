Add a test to the existing `#[cfg(test)]` module in `zbobr-api/src/config.rs` that exercises the full round-trip: parse two TOML config snippets → merge them → verify Vec fields behave correctly.

**What to test:**
- Parse a base TOML string defining a workflow with roles (including `mcp` list) and pipeline stages (including `prompts` list)
- Parse an overlay TOML string that: (a) overrides one role's `mcp` with a different list, (b) leaves another role's `mcp` absent (should inherit), (c) explicitly clears a stage's `prompts` with `prompts = []`
- Merge the two `WorkflowToml` structs using `merge_toml`
- Assert: overridden `mcp` has new values, inherited role preserves base `mcp`, cleared `prompts` is `Some(vec![])`

**Why:** This is the integration test that validates the full pipeline from TOML text → deserialization → merge, catching any mismatch between serde behavior and merge logic. The previous two tests verify the pieces in isolation; this one verifies they compose correctly.

**Pattern to follow:** Use `toml::from_str::<WorkflowToml>(...)` with inline TOML strings. The `[workflow]` section wrapper may or may not be needed depending on whether `WorkflowToml` is a top-level table — check how existing tests handle this. The existing tests in lines 1788+ construct structs directly, but this test should start from TOML text to exercise the full path.