In `zbobr/src/commands.rs`, in the `run()` function (around line 204-209), before creating backends, append the instance name to both directory paths:

- `dispatcher_config.workspaces` should be joined with `&dispatcher_config.instance`
- `repo_config.repos_dir` should be joined with `&dispatcher_config.instance`

Note: both `dispatcher_config` and `repo_config` parameters need to be made mutable. `dispatcher_config` is already passed by value; `repo_config` is also by value. Add `mut` to the `repo_config` parameter binding.

Place these path adjustments right after `tasks_config.instance = dispatcher_config.instance.clone();` (line 206), before the backends are constructed. This follows the existing pattern of modifying configs at the wiring point.

The analog is `tasks_config.instance = dispatcher_config.instance.clone();` on line 206 — we're doing the same kind of config adjustment at the same location.