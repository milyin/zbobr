## Test: `sample_task_and_comments` returns non-trivial values

**File:** `zbobr-dispatcher/src/prompts.rs`, inside the existing `mod tests` block

**What to test:**

Add a `#[test]` function (e.g., `sample_task_and_comments_has_nontrivial_fields`) that calls `sample_task_and_comments()` and asserts:

1. `task.pr_url` is `Some` (task has a PR URL)
2. `task.signal` is `Some` (task has a signal)
3. `task.stack` is non-empty (task has at least one stack entry)
4. `task.context.stages` is non-empty (task context has at least one stage with records)
5. All returned comments have `url` set to `Some` (comment URL fields are populated)

**Why this test is needed:**

`sample_task_and_comments()` is a public API. The task requirement explicitly asked that url fields, context, signal, and stack be filled with non-trivial values. None of the existing tests verify these field-level invariants directly — the `validate_all_prompts` tests only check that template rendering succeeds, which would pass even if all of these fields were reset to `None` / empty.

This test documents the contract and prevents silent regressions.
