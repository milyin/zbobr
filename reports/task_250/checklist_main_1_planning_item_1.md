In `zbobr/src/init.rs`, function `default_workflow()` (around lines 256-314), insert a new "linting" stage in the `main_stages` IndexMap **before** the "testing" stage.

**What:** Add stage definition:
- Stage name: "linting"
- role: "linter"
- prompts: task_prompt.clone() (same as other stages)
- on_failure: StageTransition::stage("working") (same as testing — failures go back to the worker)

**Why:** Separating linting from testing allows using cheaper models for mechanical formatting checks and gives clearer separation of concerns.

**Resulting stage order:** planning → working → reviewing → test_planner → test_worker → **linting** → testing