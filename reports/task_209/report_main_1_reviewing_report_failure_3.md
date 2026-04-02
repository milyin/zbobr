## Overall assessment

The branch mostly follows the approved analog well: provider inheritance is centralized in config resolution, dispatcher-side selection handles priority/round-robin/exclusion, workflow config now resolves a named tool, and the `Model` wrapper is back in the type system. I did not find unrelated changes outside the refactor/test surface.

I found two remaining correctness issues that should be fixed before approval.

## Findings

### 1. Account-limit / quota failures still do **not** trigger provider exclusion fallback

**Where**
- `zbobr-dispatcher/src/cli.rs:574-577`
- `zbobr-dispatcher/src/cli.rs:1573-1589`
- `zbobr-executor-claude/src/lib.rs:141-144`
- `zbobr-executor-copilot/src/lib.rs:123-126`

**Problem**
The task requirement says a provider should be excluded when it fails because of connectivity **or account limits**. The current implementation only excludes when `executor.execute(...)` returns `Err(...)`, which corresponds to spawn / I/O / connectivity-level failures:

- non-zero process exit -> `connectivity_failure: false`
- I/O/spawn failure -> `connectivity_failure: true`

Both executors reduce every non-zero exit to `Ok(ExecutorOutput { exit_ok: false, ... })`, and `execute_tool()` turns that into `execution_error` with `connectivity_failure: false`. There is no parsing/classification of quota / rate-limit / account-limit failures from executor output, so those failures never trigger exclusion.

**Why it matters**
This leaves the fallback behavior incomplete relative to the task requirements: an exhausted provider can keep being selected again and again instead of being temporarily removed from rotation.

**Suggested fix**
Classify quota/account-limit failures separately from generic task failures. For example:
1. propagate a structured failure kind from executors, or
2. inspect stderr/stdout for known quota/rate-limit/account-limit signatures before setting `connectivity_failure` / exclusion.

The key point is that quota/account-limit failures should be treated like provider unavailability for fallback purposes, while ordinary task failures should not.

### 2. Stage-title parsing now silently drops malformed model tokens instead of rejecting the stage header

**Where**
- `zbobr-api/src/context/stage_title.rs:165-169`
- `zbobr-api/src/context/mod.rs:563-577`

**Problem**
`MdStageTitle::from_str` used to fail when the second backtick token was present but was not a valid model. After this refactor it does:

```rust
model = value.parse::<Model>().ok();
```

That means an invalid model token is silently converted into `None` and parsing continues. Because context parsing treats any successfully parsed stage title as authoritative, a malformed persisted stage header now round-trips as a valid stage with the model silently erased instead of being rejected as malformed.

**Why it matters**
This is a data-loss regression in the context parser: malformed stage headers are no longer rejected cleanly, and the persisted model can disappear during parse/serialize flows.

**Suggested fix**
Restore strict parsing for the model token, e.g. return an error when a second non-timestamp backtick token is present but `Model::from_str` fails. With the current "models never contain whitespace" rule, there is no need to silently downgrade this case to `None`.

## Analog consistency

The overall structure is consistent with the planned analog and with surrounding code patterns. The remaining issues are both boundary-condition problems rather than architectural mismatches:
1. fallback classification does not yet cover the full required failure class, and
2. the stricter `Model` invariant is not enforced consistently in stage-title parsing.

## Checklist status

All checklist items in the provided context were already marked complete. I did not find any remaining unchecked items to mark; this failure report is due to the two correctness gaps above.