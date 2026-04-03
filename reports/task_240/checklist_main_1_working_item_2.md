In zbobr/src/commands.rs Show{id: None, json: true} path:
- Change to serialize Vec<Task> with serde_json::to_string_pretty(&tasks)
- Remove TaskListEntry::from conversion in that path