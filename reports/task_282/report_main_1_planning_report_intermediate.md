## Plan: reuse sample task code

### Problem
`dummy_task_and_comments()` in `zbobr/src/commands.rs` and the inline dummy `Task` in `validate_all_prompts()` (`zbobr-dispatcher/src/prompts.rs`) are duplicate constructions of sample data. The task asks to unify them, rename the function, and enrich it with non-trivial field values.

### Approach

1. **Move and rename** the function from `zbobr/src/commands.rs` to `zbobr-dispatcher/src/prompts.rs` (near `validate_all_prompts()`), renaming it `sample_task_and_comments()`. Make it `pub` and re-export from `zbobr-dispatcher/src/lib.rs`.

2. **Enrich the sample data** with non-trivial values for all currently empty/default fields:
   - `pr_url`: `Some("https://github.com/owner/repo/pull/1".to_string())`
   - `context`: `TaskContext` with one `StageContext` containing a `ContextRecord`
   - `signal`: `Some(Signal::Go("some-stage".into()))`
   - `stack`: vec with one `StackEntry`
   - Comment `url`: `Some("https://github.com/owner/repo/issues/1#issuecomment-123".to_string())`

3. **Update `validate_all_prompts()`** to call `sample_task_and_comments()` instead of constructing its own inline `Task`.

4. **Update `zbobr/src/commands.rs`** to remove the old function and use `sample_task_and_comments()` from `zbobr_dispatcher`.

### Why this approach
- Moves the function to `zbobr-dispatcher` since the dependency direction requires it (binary crate `zbobr` depends on `zbobr-dispatcher`, not the reverse).
- Placing it in `prompts.rs` keeps it close to its primary consumer (`validate_all_prompts`).
- Non-trivial values for `context`, `signal`, `stack`, and `pr_url` mean the sample data exercises more prompt template variables, making validation more meaningful.
