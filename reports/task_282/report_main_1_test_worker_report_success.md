# Test Implementation: sample_task_and_comments_has_nontrivial_fields

## What was done

Added `#[test] fn sample_task_and_comments_has_nontrivial_fields` to the `mod tests` block in `zbobr-dispatcher/src/prompts.rs`.

## Assertions

The test calls `sample_task_and_comments()` and verifies:
1. `task.pr_url` is `Some`
2. `task.signal` is `Some`
3. `task.stack` is non-empty
4. `task.context.stages` is non-empty, each stage has at least one record
5. All returned comments have `url` set to `Some`

## Test result

```
test prompts::tests::sample_task_and_comments_has_nontrivial_fields ... ok
test result: ok. 1 passed; 0 failed
```

## Commit

`13d1b3e2` — test: add sample_task_and_comments_has_nontrivial_fields unit test
