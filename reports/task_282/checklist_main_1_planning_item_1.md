## Update validate_all_prompts to use sample_task_and_comments

### What to do
In `zbobr-dispatcher/src/prompts.rs`, update the `validate_all_prompts` method to call `sample_task_and_comments()` instead of constructing its own inline `Task` and empty `comments` vector.

Replace:
```
let task = Task { ... inline construction ... };
let comments: Vec<Comment> = vec![];
```
with:
```
let (task, comments) = sample_task_and_comments();
```

### Why
Issue #266 added `validate_all_prompts` to validate prompt templates. Using `sample_task_and_comments()` with non-trivial values means the validation exercises more template variables (context, signal, stack, pr_url, comment urls), making it more meaningful — catching template bugs that only trigger when these fields are populated.