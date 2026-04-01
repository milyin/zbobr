In `README.md` lines 98-113, the example TOML config incorrectly shows `task_repo = "owner/repo"` in `[dispatcher]`. The correct config for the task backend is `github_repo = "owner/..."` in `[tasks]`. Also line 20 says `tasks.task_repo` which should be `tasks.github_repo`. The description on line 100 omits the `[tasks]` section.

Fix:
1. Line 20: `tasks.task_repo` → `tasks.github_repo`
2. Example TOML block: replace `[dispatcher]\ntask_repo = "owner/repo"` with correct `[tasks]\ngithub_repo = "owner/zbobr-test-tasks"` section
3. Update the description to mention all 3 sections: `[dispatcher]`, `[tasks]`, and `[repo]`