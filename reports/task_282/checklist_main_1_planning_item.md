## Move and rename dummy_task_and_comments → sample_task_and_comments

### What to do
1. In `zbobr-dispatcher/src/prompts.rs`, add a new `pub fn sample_task_and_comments() -> (Task, Vec<Comment>)` function (similar to the existing private `dummy_task` helper but returning both task and comments).
2. Fill in non-trivial values for all previously empty/default fields:
   - `pr_url`: a sample GitHub PR URL string wrapped in `Some(...)`
   - `context`: a `TaskContext` with at least one `StageContext` entry containing a `ContextRecord` (e.g., a `Comment` or `Success` type record with some text)
   - `signal`: `Some(Signal::Go("some-stage".into()))` or similar — use a real `Signal` variant from `zbobr_api::task`
   - `stack`: a vec with one `StackEntry` (inspect the struct fields and fill with sample values)
   - Comment `url`: `Some("https://github.com/owner/repo/issues/1#issuecomment-123".to_string())`
3. Export this function from `zbobr-dispatcher/src/lib.rs` (add to the existing `pub use prompts::{...}` block).
4. In `zbobr/src/commands.rs`, remove the old `dummy_task_and_comments` function definition and replace all call sites with `zbobr_dispatcher::sample_task_and_comments()` (the import is already available via the dispatcher crate).

### Why
The function is duplicated in two places. Moving it to `zbobr-dispatcher` is correct because the dependency direction allows `zbobr` (binary) to depend on `zbobr-dispatcher` (library), but not the reverse. Exporting it makes it accessible to both consumers.