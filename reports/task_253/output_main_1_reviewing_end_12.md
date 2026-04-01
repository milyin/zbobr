I’m reviewing the branch end-to-end against `main`, then I’ll spot-check the changed files for consistency with the single-repo simplification.

● Get full task diff (shell)
  │ git --no-pager diff --unified=0 origin/main...HEAD
  └ 10 lines...

● Inspect commits and changed files (shell)
  │ git --no-pager log --oneline origin/main..HEAD && printf '\n---DIFFSTAT---\n' && git --no-pager
  │ diff --stat origin/main...HEAD && printf '\n---FILES---\n' && git --no-pager diff --name-only
  │ origin/main...HEAD
  └ 111 lines...

I’ve got the change set. Next I’m checking the core surfaces the task touched: repo backends, dispatcher/task identity, dry-run prompt wiring, and docs/examples.

● Read github.rs
  │ zbobr-repo-backend-github/src/github.rs
  └ L1:320 (320 lines read)

● Read config.rs
  │ zbobr-repo-backend-github/src/config.rs
  └ L1:260 (181 lines read)

● Read config.rs
  │ zbobr-repo-backend-fs/src/config.rs
  └ L1:240 (138 lines read)

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L1:260 (260 lines read)

● Read backend.rs
  │ zbobr-api/src/backend.rs
  └ L1:260 (260 lines read)

● Read fs.rs
  │ zbobr-repo-backend-fs/src/fs.rs
  └ L1:260 (260 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L1:320 (320 lines read)

● Read README.md
  └ L1:320 (320 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1:340 (340 lines read)

● Read zbobr_github_test.toml.sample
  └ L1:240 (32 lines read)

● Read init.rs
  │ zbobr/src/init.rs
  └ L1:320 (320 lines read)

● Read example-task.yaml
  │ zbobr-task-backend-fs/example-task.yaml
  └ L1:220 (21 lines read)

I’m doing a targeted sweep for stale multi-repo and preparator references, plus places where normalization could still diverge between preview and runtime.

● Search (grep)
  │ "parse_github_repo|normalize_github_repo|repo_short_name|ensure_pr_url|repository\(|branch\(|Tas
  │ kIdentity|work_branch_prefix" in !target/**
  └ 5 lines found

● Search (grep)
  │ "fork_owner|default_destination_repository|default_destination_branch|destination_repository|des
  │ tination_branch|tasks-github-task-repo|repo-github-repository|preparator|preparation|go_preparat
  │ ion|PREPARING" in !target/**
  └ 13 lines found

● Read lib.rs
  │ zbobr-executor-mcp-tester/src/lib.rs
  └ L130:180 (38 lines read)

● Read copilot-tool-output-1775052077458-j8djaw.txt
  │ /tmp/copilot-tool-output-1775052077458-j8djaw.txt
  └ L1:260 (127 lines read)

● Read init.rs
  │ zbobr/src/init.rs
  └ L640:720 (65 lines read)

● Read abstract_test_helpers.rs
  │ zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs
  └ L450:490 (41 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1280:1325 (46 lines read)

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L960:1070 (111 lines read)

● Read task.rs
  │ zbobr-dispatcher/src/task.rs
  └ L1:240 (240 lines read)

● Read prompts.rs
  │ zbobr-dispatcher/src/prompts.rs
  └ L1:220 (220 lines read)

● Read github.rs
  │ zbobr-task-backend-github/src/github.rs
  └ L1:220 (220 lines read)

● Read fs.rs
  │ zbobr-task-backend-fs/src/fs.rs
  └ L1:220 (220 lines read)

● Search (grep)
  │ "task_repo|github_repo|repo_reposit|repo-repository|tasks.github|backend_github|force
  │ push|fork|multi-repo|multiple repos|any set of repositories|destination repo|target
  │ repo|reference a target repo|default model" (README.md)
  └ 9 lines found

● Search (grep)
  │ "destination_repository|destination_branch|fork_owner|default_destination_repository|default_des
  │ tination_branch" in !target/**
  └ 7 lines found

● Search (grep)
  │ "task_repo|github_repo|repo-repository|tasks.github|backend_github|fork|preparator|preparation|P
  │ REPARING|destination_repository|destination_branch" (docs)
  └ 1 line found

● Search (grep)
  │ "identity\(\)|TaskIdentity \{|work_branch: self.work_branch" in !target/**
  └ 12 lines found

● Search (grep)
  │ "ensure_work_branch\(" in !target/**
  └ 2 lines found

✗ Search (grep)
  │ "--tasks-github-repo|--repo-repository|github_repo =|repository =" (docs)
  └ rg: unrecognized flag --tasks-github-repo|--repo-repository|github_repo

● Read task.rs
  │ zbobr-dispatcher/src/task.rs
  └ L500:560 (61 lines read)

● Read README.md
  └ L320:380 (42 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L340:420 (81 lines read)

● Search (grep)
  │ "tasks-github-repo|repo-repository|github_repo =|repository =" (docs)
  └ 1 line found

● Search (grep)
  │ "CliStageRunner|run_stage|ensure_pr_url\(|update_worktree\(|task_session\(.*finish|process_task|
  │ loop_once|run_loop" (zbobr-dispatcher/src)
  └ 19 lines found

● Search (grep)
  │ "ZbobrRepoBackendGithubConfig|ZbobrRepoBackendFsConfig|RootConfigToml|\[repo\]|\[tasks\]"
  │ (zbobr/src)
  └ 9 lines found

● Read main.rs
  │ zbobr/src/main.rs
  └ L1:180 (103 lines read)

● Read github-token-permissions.md
  │ docs/github-token-permissions.md
  └ L1:120 (49 lines read)

● Read github.rs
  │ zbobr-repo-backend-github/src/github.rs
  └ L620:860 (241 lines read)

● Search (grep)
  │ "default_destination_repository|default_destination_branch|fork_owner" in !target/**
  └ No matches found

● Search (grep)
  │ "agent_github_token|ZBOBR_AGENT_GH_TOKEN|ZBOBR_OWNER_GH_TOKEN|ZBOBR_REPO_GITHUB_TOKEN|ZBOBR_TASK
  │ _GITHUB_TOKEN" in !target/**
  └ 30 lines found

● Search (grep)
  │ "github-token-permissions|ZBOBR_REPO_GITHUB_TOKEN|ZBOBR_TASK_GITHUB_TOKEN|ZBOBR_AGENT_GH_TOKEN|Z
  │ BOBR_OWNER_GH_TOKEN" (README.md)
  └ 7 lines found

● Search (grep)
  │ "extract_repo_name\(" in !target/**
  └ 1 line found

● Search (grep)
  │ "tasks-github-repo|repo-repository|agent_token" (README.md)
  └ 2 lines found

● Read config.rs
  │ zbobr-api/src/config.rs
  └ L560:610 (51 lines read)

● Search (grep)
  │ "agent_github_token.resolve|different from|must be set|GH_TOKEN|GITHUB_TOKEN"
  │ (zbobr-api/src/config.rs)
  └ 2 lines found

● Read README.md
  │ zbobr-task-backend-fs/README.md
  └ L1:220 (88 lines read)

● Search (grep)
  │ "legacy|migrate legacy|stage field" in !target/**
  └ 12 lines found

● Search (grep)
  │ "stage:|state:|example-task.yaml|work_branch|pr_url" (zbobr-task-backend-fs)
  └ 16 lines found

● Read zbobr_github_test.toml.sample
  └ L24:40 (9 lines read)

● Search (grep)
  │ "agent_token\s*=|agent_github_token\s*=|ZBOBR_AGENT_GH_TOKEN|ZBOBR_REPO_GITHUB_TOKEN|ZBOBR_TASK_
  │ GITHUB_TOKEN" in !target/**
  └ 9 lines found

One more pass: I found a few documentation mismatches already. I’m checking whether the GitHub repo parser still accepts malformed “copy-pasted” URLs that would break the runtime.

● Read github.rs
  │ zbobr-repo-backend-github/src/github.rs
  └ L900:1055 (155 lines read)

● Probe parser edge cases (shell)
  │ python - <<'PY'
  │ from urllib.parse import urlparse
  │ samples=[
  │ 'https://github.com/owner/repo?tab=readme',
  │ 'https://github.com/owner/repo#readme',…
  └ 10 lines...

● Demonstrate parse edge cases (shell)
  │ node - <<'JS'
  │ const samples=[
  │ 'https://github.com/owner/repo?tab=readme',
  │ 'https://github.com/owner/repo#readme',
  │ 'https://github.com/owner/repo.git?tab=readme',…
  └ 13 lines...

● report_failure
  └ Report stored

Review failed.

I found 2 must-fix issues:

- `parse_github_repo()` is still too permissive for HTTPS URLs with query strings or fragments. Inputs like `https://github.com/owner/repo?tab=readme` currently normalize to invalid repo names like `owner/repo?tab=readme`, which can break both dry-run preview and runtime API calls.

- Docs/examples are still inconsistent with the shipped interfaces. In particular, `README.md` still documents unsupported `ZBOBR_*` token names, `zbobr_github_test.toml.sample` still uses `agent_token` instead of `agent_github_token`, and `zbobr-task-backend-fs/README.md` plus `example-task.yaml` still describe the old legacy task schema instead of the current `state`/`work_branch`/`context` model.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         7m 10s
Total session time:     7m 40s
Total code changes:     +0 -0
Breakdown by AI model:
 gpt-5.4                  1.6m in, 9.8k out, 1.5m cached (Est. 1 Premium request)