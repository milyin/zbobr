## Test Planning Report

### Summary

The implementation changes rename `dummy_task_and_comments` → `sample_task_and_comments`, enrich it with non-trivial values, make it public in `zbobr-dispatcher`, and consolidate its use across `validate_all_prompts` and CLI command handlers.

### Existing coverage assessment

The `validate_all_prompts` function already has extensive tests (valid templates, undefined variables, missing files, multi-error aggregation, multi-pipeline, call-stage skipping). These tests exercise `sample_task_and_comments()` indirectly, because `validate_all_prompts` now calls it. They confirm that template rendering succeeds with the sample data.

### Gap identified

No test directly asserts that the non-trivial fields required by the task (`pr_url`, `signal`, `stack`, `context.stages`, comment `url` fields) are actually populated. A regression — e.g., resetting `pr_url` to `None` or clearing the context — would not be caught by existing tests, because template rendering succeeds regardless of whether those fields are set.

### Test plan

**1 test / 1 checklist item:**

| Test | Location | Purpose |
|------|----------|---------|
| `sample_task_and_comments_has_nontrivial_fields` | `zbobr-dispatcher/src/prompts.rs` (mod tests) | Assert that `pr_url`, `signal`, `stack`, `context.stages`, and comment `url` fields are all `Some`/non-empty |

No additional test groups are needed. The existing `validate_all_prompts` tests cover prompt-rendering correctness; the single new test covers the data-contract correctness of the sample helper.
