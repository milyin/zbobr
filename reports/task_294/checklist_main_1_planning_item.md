In `zbobr/src/init.rs`, find the `linting` stage definition in the `main_stages` IndexMap. Change its `on_failure` transition from `"working"` to `"linter_worker"`.

**Why**: This is the core routing change — instead of escalating to the general worker when linting fails, the task should go to the dedicated `linter_worker` stage for a tight fix loop.

**Pattern to follow**: Look at how `test_planner` routes to `test_worker` on `on_intermediate` as an analog for how one stage routes to a specialized worker stage.

**Constraint**: The `on_success` of `linting` remains unchanged (goes to the next stage in the pipeline).