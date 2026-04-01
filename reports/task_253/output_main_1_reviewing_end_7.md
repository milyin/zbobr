I’m reviewing the branch holistically now: first I’ll map the full change set against `main`, then I’ll spot-check the simplified single-repo flow and scan for any lingering multi-repo/preparator surface area.

● Search (grep)
  │ "preparator|preparation|fork_owner|default_destination_repository|default_destination_branch|des
  │ tination_repository|destination_branch" in *.{rs,md,toml}
  └ 30 lines found

● Inspect commits and changed files (shell)
  │ git --no-pager log --oneline origin/main..HEAD && printf '\n---FILES---\n' && git --no-pager
  │ diff --name-only origin/main...HEAD && printf '\n---STAT---\n' && git --no-pager diff --stat
  │ origin/main...HEAD
  └ 97 lines...

I’ve got the overall diff. Next I’m reading the core files that changed the routing model plus the spots still mentioning old concepts, to separate acceptable compatibility shims from real leftover multi-repo/preparator behavior.

● Read task.rs
  │ zbobr-api/src/task.rs
  └ 1 line read

● Read github.rs
  │ zbobr-repo-backend-github/src/github.rs
  └ L1:420 (420 lines read)

● Read lib.rs
  │ zbobr-executor-mcp-tester/src/lib.rs
  └ L120:200 (48 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1:420 (420 lines read)

● Read backend.rs
  │ zbobr-api/src/backend.rs
  └ 281 lines read

● Read fs.rs
  │ zbobr-repo-backend-fs/src/fs.rs
  └ 265 lines read

● Read prompts.rs
  │ zbobr-dispatcher/src/prompts.rs
  └ 1 line read

● Read config.rs
  │ zbobr-repo-backend-fs/src/config.rs
  └ 138 lines read

● Read config.rs
  │ zbobr-repo-backend-github/src/config.rs
  └ 181 lines read

● Read README.md
  └ L1:260 (260 lines read)

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L1:260 (260 lines read)

● Read zbobr_github_test.toml.sample
  └ 32 lines read

I found at least one documentation inconsistency already. I’m now checking whether the remaining old names are just generic prompt variables/tests, or if any real code path still behaves like multi-repo/preparator mode exists.

● Read github-token-permissions.md
  │ docs/github-token-permissions.md
  └ 49 lines read

● Read transitions.md
  │ docs/transitions.md
  └ 45 lines read

● Read config.rs
  │ zbobr-executor-mcp-tester/src/config.rs
  └ 81 lines read

● Search (grep)
  │ "VAR_DESTINATION_REPOSITORY|VAR_DESTINATION_BRANCH|task_repo|target repo|target
  │ repository|multiple repos|multiple repositories|reference a target
  │ repo|preparation|preparator|fork_owner" in *.{rs,md,toml}
  └ 58 lines found

● Read transitions.dot
  │ docs/transitions.dot
  └ 81 lines read

● Read init.rs
  │ zbobr/src/init.rs
  └ L360:760 (269 lines read)

● Read README.md
  └ L260:420 (99 lines read)

● Read task.rs
  │ zbobr-dispatcher/src/task.rs
  └ L1:260 (260 lines read)

● Read lib.rs
  │ zbobr-dispatcher/src/lib.rs
  └ L1:220 (220 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L1:280 (280 lines read)

● Read env.rs
  │ zbobr-dispatcher/tests/mcp_integration/env.rs
  └ L150:260 (111 lines read)

● Read test_helpers.rs
  │ zbobr-dispatcher/tests/mcp_integration/test_helpers.rs
  └ L760:1095 (336 lines read)

I found a stronger issue than docs: the GitHub integration test harness still models fork/cross-org behavior and per-task target repos. I’m checking whether that’s dead code or still part of the supported test surface before I write it up.

● Read env.rs
  │ zbobr-dispatcher/tests/mcp_integration/env.rs
  └ L260:420 (161 lines read)

● Read abstract_test_helpers.rs
  │ zbobr-dispatcher/tests/mcp_integration/abstract_test_helpers.rs
  └ L100:150 (51 lines read)

● Read integration_github_github.rs
  │ zbobr-dispatcher/tests/integration_github_github.rs
  └ 130 lines read

● Search (grep)
  │ "fn fork_owner|fork_owner\(|update_task_branches\(|CROSS_ORG_DEST_REPO|run_repo_backend_clone_cr
  │ oss_org|run_repo_backend_planning_cross_org|destination_repository|destination_branch" in *.rs
  │ (./zbobr-dispatcher/tests)
  └ 57 lines found

● Search (grep)
  │ "run_repo_backend_.*cross_org|run_repo_backend_clone_cross_org|fork_owner"
  │ (./zbobr-dispatcher/tests/integration_github_github.rs)
  └ No matches found

● Search (grep)
  │ "struct GitHubTestConfig|fork_owner" in *.rs (./zbobr-dispatcher/tests/mcp_integration)
  └ 17 lines found

● Read integration_github_github.rs
  │ zbobr-dispatcher/tests/integration_github_github.rs
  └ L130:320 (1 line read)

● Read github_config.rs
  │ zbobr-dispatcher/tests/mcp_integration/github_config.rs
  └ 56 lines read

● Search (grep)
  │ "run_repo_backend_clone\(|run_repo_backend_planning\(|run_repo_backend_working\(|run_repo_backen
  │ d_reviewing\(|run_repo_backend_merging\(" in *.rs (./zbobr-dispatcher/tests)
  └ 5 lines found

● Search (grep)
  │ "task_repo =|github_repo =|repository = \"owner/target-repo\"|reference a target repo|target
  │ repository|target repo" (./README.md)
  └ 6 lines found

● Search (grep)
  │ "extract_repo_name\(" in *.rs
  └ 1 line found

● Search (grep)
  │ "go_preparation|preparation" in *.rs (./zbobr/src)
  └ 1 line found

● Search (grep)
  │ "preparation|preparator" in *.rs (./zbobr-executor-mcp-tester)
  └ 1 line found

● Read config.rs
  │ zbobr-dispatcher/src/config.rs
  └ L1:160 (29 lines read)

● Read config.rs
  │ zbobr-task-backend-github/src/config.rs
  └ L1:120 (67 lines read)

✗ Search (grep)
  │ "task_repo" in *.rs (./zbobr-api ./zbobr-dispatcher ./zbobr-task-backend-github ./zbobr)
  └ rg: ./zbobr-api ./zbobr-dispatcher ./zbobr-task-backend-github ./zbobr: IO error for operation
    on ./zbobr-api ./zbobr-dispatcher ./zbobr-task-backend-github ./zbobr: No such file or directory
    (os error 2)

● Read github.rs
  │ zbobr-repo-backend-github/src/github.rs
  └ L420:760 (341 lines read)

● Search (grep)
  │ "parse_github_repo\(|Invalid GitHub URL|Invalid GitHub repository format|github.com"
  │ (./zbobr-repo-backend-github/src/github.rs)
  └ 22 lines found

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L340:460 (120 lines read)

✗ Search (grep)
  │ "fn identity\(|TaskIdentity \{" in *.rs (./zbobr-api/src/task.rs ./zbobr-dispatcher/src
  │ ./zbobr-task-backend-* ./zbobr-repo-backend-* ./zbobr/src)
  └ rg: ./zbobr-api/src/task.rs ./zbobr-dispatcher/src ./zbobr-task-backend-* ./zbobr-repo-backend-*
    ./zbobr/src: IO error for operation on ./zbobr-api/src/task.rs ./zbobr-dispatcher/src
    ./zbobr-task-backend-* ./zbobr-repo-backend-* ./zbobr/src: No such file or directory (os error
    2)

● Search (grep)
  │ "fn identity\(|TaskIdentity \{" in *.rs
  └ 3 lines found

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L680:760 (81 lines read)

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L1024:1048 (25 lines read)

● Search (grep)
  │ "CROSS_ORG_DEST_REPO|fork remote|fork_owner|cross-org|clone_and_setup|same-org|target_repo not
  │ configured" (./zbobr-dispatcher/tests/mcp_integration/test_helpers.rs)
  └ 42 lines found

✗ Search (grep)
  │ "fork|default_destination_repository|default_destination_branch|destination_repository|destinati
  │ on_branch|task_repo|target repo|target repository|multiple repos|multiple repositories|Task
  │ Projects" in *.{rs,md} (./zbobr/src/init.rs ./docs ./README.md
  │ ./zbobr-task-backend-fs/README.md)
  └ rg: ./zbobr/src/init.rs ./docs ./README.md ./zbobr-task-backend-fs/README.md: IO error for
    operation on ./zbobr/src/init.rs ./docs ./README.md ./zbobr-task-backend-fs/README.md: No such
    file or directory (os error 2)

● report_failure
  └ Report stored

Review failed: I found 3 must-fix issues.

First, `parse_github_repo()` is overpermissive and can silently reinterpret non-GitHub URLs as `github.com/owner/repo`, which is a real correctness bug in the new normalization path.

Second, `zbobr task update --help` still exposes `go_preparation` in the `--signal` help text, so the removed preparator stage is still present in a public interface.

Third, the single-repo cleanup is incomplete in docs/tests: `README.md` still describes task-selected target repos and uses outdated config names, and the GitHub integration helpers still model per-task destination repo/branch plus cross-org fork behavior.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         3m 58s
Total session time:     4m 27s
Total code changes:     +0 -0
Breakdown by AI model:
 gpt-5.4                  1.3m in, 8.9k out, 1.2m cached (Est. 1 Premium request)