## Overall assessment
The branch is narrowly scoped and the main implementation style is consistent with the chosen analog in `zbobr-dispatcher/src/cli.rs`: helper-style projection/selection functions and thin command dispatch in `zbobr/src/commands.rs`. I did not find unrelated file churn.

However, there is still one blocking correctness issue against the task requirements and the command’s own documented behavior.

## Finding

### 1. `task show --json` without an ID still bypasses the real task-listing code
**Files:**
- `zbobr/src/commands.rs:165-177`
- `zbobr/src/commands.rs:258-267`
- `zbobr/src/commands.rs:360-387`
- Context intent: `ctx_rec_10`

`TaskSubcommand::Show { id: None, .. }` is still classified as **not needing backends** in `Command::needs_backends()`:

```rust
Command::Task {
    subcommand: TaskSubcommand::Show { id: None, .. },
}
```

Because of that, `run()` routes `zbobr task show` with no ID into `run_without_backends()`, where it always prints the sample task:

```rust
Command::Task {
    subcommand: TaskSubcommand::Show { id: None, json },
} => {
    let (task, comments) = sample_task_and_comments();
    if json {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        print_task(&task, &comments);
    }
}
```

The new backend-aware branch that lists all tasks and, for `--json`, serializes `Vec<Task>` is therefore unreachable in normal execution:

```rust
TaskSubcommand::Show { id, json } => {
    ...
    } else {
        let weak_tasks = task_backend.list_tasks().await?;
        ...
        if json {
            println!("{}", serde_json::to_string_pretty(&tasks)?);
        }
```

So the branch contains the right serialization code for the no-ID JSON case, but the command dispatcher still prevents users from reaching it. In practice, `zbobr task show --json` without an ID will return the sample task JSON instead of all real tasks.

### Why this matters
This directly contradicts:
1. the subcommand documentation (`Show a task by ID (or list all tasks if no ID given)`), and
2. the intended fix recorded in `ctx_rec_10` (`serialize Vec<Task> with serde_json::to_string_pretty(&tasks)` for `Show{id: None, json: true}`).

So the task is not fully complete yet.

### Suggested fix
Remove `TaskSubcommand::Show { id: None, .. }` from the `needs_backends()` no-backend fast path so normal `task show` without an ID goes through `run_task_subcommand()` and can list real tasks / emit real task JSON.

If the sample-task behavior is still useful for prompt/demo purposes, it should be exposed through a separate explicit preview/demo path rather than the main `task show` command.

## Analog consistency
The analog choice was appropriate, and the new code generally follows the existing helper-based style. The remaining problem is not stylistic; it is routing: the dispatcher still sends the no-ID `show` command to the sample-only path instead of the backend-backed implementation.

## Extraneous changes
I did not find unrelated modifications in this branch.

## Checklist
All checklist items were already marked complete in context, so I did not check any additional items during this review.