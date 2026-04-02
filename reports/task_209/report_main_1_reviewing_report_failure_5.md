## Overall assessment

The main implementation follows the planned analog well: provider inheritance is centralized in config resolution, dispatcher-side provider selection handles priority/round-robin/exclusion, executor construction is keyed off resolved providers, and the `Model` wrapper is consistently restored through config/context boundaries. I did not find unrelated production changes outside the refactor surface.

I found one remaining correctness issue in the changed test/integration plumbing that should be fixed before approval.

## Finding

### Integration test dispatcher configs still use pre-refactor `tool` values and skip startup validation

**Where**
- `zbobr-dispatcher/tests/mcp_integration/env.rs:57-64`
- `zbobr-dispatcher/tests/mcp_integration/env.rs:120-129`
- `zbobr-dispatcher/tests/mcp_integration/env.rs:143-149`
- `zbobr-dispatcher/tests/mcp_integration/env.rs:187-190`
- `zbobr-dispatcher/tests/mcp_integration/env.rs:217-226`
- `zbobr-dispatcher/tests/mcp_integration/env.rs:238-244`
- `zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs:60-62`
- `zbobr-dispatcher/src/cli.rs:349-350`
- `zbobr/src/commands.rs:225-235`

**Problem**
Production now treats `tool` as a named entry in `[tools]`, which is then resolved to `(provider, model)` via `select_provider()`. That is correctly enforced in the real startup path because `zbobr/src/commands.rs` builds the dispatcher and then calls `.validated()?`.

The integration helpers were only partially migrated:
1. they still assign `tool = "mcp-tester"` directly in dispatcher config and stage definitions, using the old meaning of `tool` as an executor name;
2. they do not define matching `[providers]` / `[tools]` entries for `mcp-tester`; and
3. they build dispatchers with `.build()` only, without `.validated()`.

But the runtime stage path now does:
```rust
let tool_name = self.zbobr.config().resolve_tool_name(...);
let (resolved_provider, model) = self.zbobr.select_provider(&tool_name)?;
```
So any role stage executed through these helpers requires `tool_name` to exist in `config.tools`. With the current helper configs, `select_provider("mcp-tester")` will fail because there is no such tool definition.

**Why it matters**
This leaves the changed test harness out of sync with the production configuration model introduced by this task. It can either fail when these integration paths are exercised, or avoid catching configuration regressions because it bypasses the same startup validation that production uses.

**Suggested fix**
Update the integration helpers to use the new model end-to-end:
1. define a test provider, e.g. `providers["mcp-tester"] = { executor = "mcp-tester" }`;
2. define a named tool, e.g. `tools["mcp-tester"] = [{ provider = "mcp-tester", model = "test-model" }]`;
3. keep stage/dispatcher `tool` fields pointing to that tool name; and
4. build helper dispatchers with `.validated()` so test construction matches production startup behavior.

## Analog consistency

Aside from this remaining test-harness inconsistency, the main implementation is pattern-consistent with the planned analog and with surrounding config/dispatcher code. The issue is localized to the helper setup, not the core provider-selection refactor.