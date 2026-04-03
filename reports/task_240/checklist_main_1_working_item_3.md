In zbobr-dispatcher/src/cli.rs: rename `description: String` → `title: String` in TaskListEntry struct and its From impl (use task.title).
In zbobr/src/commands.rs line 356: change `task.description` → `task.title` in the compact list println.
User explicitly requested "restore the title! The description is a misprint."