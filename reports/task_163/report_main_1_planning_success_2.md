## Plan rationale

The reviewer identified 4 issues in the current TaskContext implementation. All are straightforward cleanup/correctness fixes:

1. **Remove checklist field** — The old `checklist: Vec<ChecklistItem>` on `Task` is fully superseded by `context: TaskContext`. ~20 locations across 8 crates still reference it and need cleanup, including the entire `checklist_format.rs` module, backend trait method, dispatcher RoleSession/TaskSession methods, and various tests.

2. **Remove user_comment** — The `user_comment: Option<String>` field on `StageContext` was part of the original design but is unused. User comments are handled separately via the `Comment` struct and interspersed during serialization by timestamp. All ~14 references just set it to `None`.

3. **Pipeline::from vs parse** — `Pipeline::from(pipeline_str)` at context_format.rs:220 delegates to `FromStr` via `.parse().unwrap()` internally. Using `.parse().unwrap()` directly is more idiomatic and consistent with how `tool` and `model` are parsed on adjacent lines. Both are equivalent since `FromStr::Err = Infallible`.

4. **Error on unrecognized lines** — `parse_record_line` returns `Ok(None)` for lines that don't match any known prefix. The caller in `parse_context` silently skips these. Instead, after all known line types are checked (empty, blockquote, stage header, record, HTML comment), any remaining non-empty line should trigger an error to catch corrupted content.

No analog needed — these are direct fixes to existing code with clear specifications from the reviewer.