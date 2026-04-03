In zbobr-dispatcher/src/cli.rs:
- Change TaskListEntry.title to TaskListEntry.description
- Update From<&Task> to use task.description.clone()
In zbobr/src/commands.rs:
- Update compact list display to use task.description instead of task.title