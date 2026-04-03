# Unit tests for `TaskListEntry`

**File:** `zbobr-dispatcher/src/cli.rs` — same test module as select_runnable_task tests

**Tests:**

1. `from_task_projects_correct_fields` — construct a Task with known id/stage_count/state/title, convert via `TaskListEntry::from(&task)`, assert all four fields match
2. `json_serialization_has_expected_keys` — serialize a `TaskListEntry` to JSON, parse as `serde_json::Value`, assert the object contains exactly the keys `id`, `stage_count`, `state`, `title` (this validates the `--json` output contract)

**Dependencies:** `serde_json`, `Task`, `TaskListEntry`, `State`