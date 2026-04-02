## Overall assessment

The refactor is mostly consistent with the approved analog: provider inheritance is centralized in config resolution, dispatcher-side selection uses priority plus round-robin, executor construction is keyed off resolved providers, `Model` is back as a wrapper type, and the recent integration-test helper fix brought test setup back in line with the production provider/tool model. I did not find unrelated code changes in the reviewed diff.

However, there are still **2 correctness gaps** that should be fixed before approval.

## 1. Provider exclusion does not actually fall back within the same stage execution

**Where**
- `zbobr-dispatcher/src/cli.rs:346-351`
- `zbobr-dispatcher/src/cli.rs:559-577`
- `zbobr-dispatcher/src/cli.rs:1573-1584`
- plan expectation in `ctx_rec_8` (“Retry-with-exclusion logic lives in cli.rs stage runner”)

**Problem**
The stage runner selects exactly one provider/model pair at the start of the stage:

```rust
let tool_name = self.zbobr.config().resolve_tool_name(...);
let (resolved_provider, model) = self.zbobr.select_provider(&tool_name)?;
```

It then runs the executor once. If that execution fails due to connectivity or quota/account-limit conditions, the provider is excluded **after** the failed run:

```rust
if outcome.connectivity_failure {
    self.zbobr.exclude_provider(&resolved_provider.name);
}
```

But there is no retry loop that re-runs the stage with the next selectable provider for the same tool. The failure still propagates through normal stage finalization for that run.

**Why it matters**
The task description explicitly defines tools as fallback lists of `(provider, model)` pairs, where a failed provider is excluded so another provider can be selected. The approved plan also called out retry-with-exclusion in the CLI stage runner. In the current implementation, exclusion only affects **later** stage executions; it does not provide the intended fallback behavior for the current stage run.

A concrete example from the task description:
- `smart = [copilot, claude, claude_pay_per_token]`
- if `copilot` fails because of connectivity/quota, the current stage should proceed by trying `claude`
- today, the current stage just fails, and only a future retry of the whole stage can use `claude`

**Suggested fix**
Wrap provider selection + executor run in a bounded retry loop inside `CliStageRunner::run()`:
1. resolve the tool name once;
2. select a provider;
3. run it;
4. if the outcome is a connectivity/quota failure, exclude that provider and immediately re-select/retry;
5. stop when execution succeeds, when a non-provider failure occurs, or when all providers for the tool are exhausted.

That would match both the task requirement and the approved analog.

## 2. Startup validation still accepts configs with no resolvable global tool

**Where**
- `zbobr-api/src/config.rs:581-583`
- `zbobr-api/src/config.rs:657-663`
- `zbobr-dispatcher/src/lib.rs:81-85`
- `zbobr-dispatcher/src/cli.rs:346-350`

**Problem**
`ZbobrDispatcherConfig::default()` still produces:

```rust
tool: "smart".to_string(),
providers: IndexMap::new(),
tools: IndexMap::new(),
```

But `validate()` only checks that the global tool exists when `self.tools` is non-empty:

```rust
if !self.tools.is_empty() && !self.tools.contains_key(self.tool.as_str()) {
    anyhow::bail!(...)
}
```

So a dispatcher with no `[tools]` section still passes `validated()`, even though any role stage that falls back to the global tool will later do:

```rust
let tool_name = resolve_tool_name(...);   // => "smart"
let (resolved_provider, model) = select_provider(&tool_name)?; // runtime failure
```

**Why it matters**
This leaves a startup-validation hole in the new config model. The task replaced the old executor/model/plan-mode triple with a single tool name that must resolve through `[tools]` and `[providers]`. A config that cannot resolve the global default tool should be rejected eagerly, not accepted at startup and then fail only when a stage runs.

This is especially important because the code already added eager validation for role/stage tool references and provider graphs; the unresolved global fallback is the remaining path that still bypasses that guarantee.

**Suggested fix**
Tighten validation so the effective global tool is always resolvable under the new model. For example:
- if role stages are supported, require `[tools]` to contain the configured global `dispatcher.tool`; or
- more simply, always require `self.tools.contains_key(self.tool.as_str())` once this refactor is active.

If an empty-tools configuration really needs to stay legal for some narrow internal use, it should be gated explicitly and never allowed through the production `.validated()` path.

## Analog consistency

Aside from the two issues above, the implementation follows the intended analog and surrounding code patterns well. The fixes should stay localized to validation and the CLI stage runner.