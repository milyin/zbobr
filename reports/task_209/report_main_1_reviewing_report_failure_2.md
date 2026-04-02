## Overall assessment

The refactor is mostly aligned with the approved analog: provider inheritance is centralized in config, dispatcher-side selection handles priority/round-robin/exclusion, workflow stages now resolve a named tool, and the `Model` wrapper is used consistently again. The earlier review findings around priority inheritance, unknown executors, tool-reference validation, and `Model` no-whitespace enforcement look fixed.

I found two remaining correctness issues that should be addressed before approval.

## Findings

### 1. Provider inheritance cycles are still not validated at startup

**Where**
- `zbobr-api/src/config.rs:615-671` — `ZbobrDispatcherConfig::validate()` checks parent existence and executor names, but does not detect circular parent chains.
- `zbobr-dispatcher/src/lib.rs:81-85` — `ZbobrDispatcher::validated()` calls `config.validate()` and `validate_workflow_refs()`, but never forces provider resolution.
- `zbobr-api/src/config.rs:724-772` — cycle detection exists only inside `resolve_providers()` / `resolve_single_provider()`.
- `zbobr-dispatcher/src/lib.rs:127` — provider resolution is deferred until `select_provider()` during stage execution.

**Problem**
A config with a provider cycle like `a -> b -> a` passes dispatcher startup validation and only fails when some stage first tries to use a tool that references one of those providers. That leaves part of the provider graph validated eagerly and part validated lazily, even though the code already has the logic needed to resolve and reject cycles.

**Why it matters**
This feature introduced a provider inheritance graph as a core configuration concept. Graph-shape errors should fail fast at startup, not in the middle of task processing after work has already begun. The current behavior is inconsistent with the rest of the config validation work in this branch, which moved other bad references to startup-time rejection.

**Suggested fix**
Make startup validation resolve providers eagerly, e.g. by calling `self.config.resolve_providers()?` from `ZbobrDispatcher::validated()` (or from `ZbobrDispatcherConfig::validate()`). That will surface circular inheritance and any future resolution-time problems before the dispatcher starts processing tasks.

### 2. Provider exclusion is triggered on every executor failure, not only connectivity/quota failures

**Where**
- `zbobr-dispatcher/src/cli.rs:575-576` — the runner excludes the selected provider whenever `outcome.execution_error.is_some()`.
- `zbobr-dispatcher/src/cli.rs:1567-1580` — `execution_error` is set for both I/O/spawn failures and any non-zero executor exit status.

**Problem**
The task requirement says a provider should be excluded when it fails due to connectivity problems or account limits. The current implementation excludes the provider after any non-zero executor exit, including ordinary task failures such as a bad prompt, an agent refusal, or a workflow/tool error unrelated to provider availability.

That means a single stage-level failure can incorrectly poison an otherwise healthy provider and force fallback to a different provider/model on the next retry.

**Why it matters**
This changes scheduler behavior in a meaningful way: retries stop being "retry the same stage" and become "retry on a different provider" even when the provider was not the problem. In practice that can mask real task failures, churn through lower-priority fallbacks unnecessarily, and burn paid providers because a stage failed for logical reasons rather than connectivity/quota reasons.

**Suggested fix**
Classify executor failures before excluding providers. At minimum, only exclude on spawn/connectivity failures or on explicit rate-limit/account-limit signals parsed from executor output. Do not exclude on generic non-zero exit status unless it is known to represent a provider-availability problem.

## Analog consistency

The high-level architecture matches the planned analog well, and the recent fixes are consistent with the surrounding code style. The remaining issues are both boundary problems:
1. provider-graph validation is still incomplete at startup, and
2. runtime fallback is broader than the task’s intended failure class.

## Checklist status

All checklist items already appear completed. I did not find any remaining unchecked checklist items to mark; the failure report is due to the correctness issues above.