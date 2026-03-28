## Populate `instance` when constructing `StageInfo` in dispatcher

Find every place where `StageInfo { ... }` is constructed in the dispatcher codebase and add `instance: self.zbobr.config().instance.clone()` (or the appropriate accessor).

**Where to look:**
- `zbobr-dispatcher/src/cli.rs`: the main stage execution code pushes a `StageContext { info: StageInfo { ... }, records: vec![] }` into `task.context.stages` — add `instance` there
- `zbobr-dispatcher/src/task.rs`: test helper code constructs `StageInfo` — add `instance: String::new()` (or a test constant) there for tests

**Why:** `StageInfo` now carries the instance name so it appears in stage title output. The dispatcher is the only producer of `StageInfo` at runtime; test code just needs a placeholder value.

**Pattern to follow:** The `pipeline`, `run_id`, `stage`, `tool`, `model`, `timestamp` fields are already populated from `self.zbobr.config()` and local context — `instance` follows the same pattern, sourced from `self.zbobr.config().instance`.