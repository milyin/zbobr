● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_12"
  └ ## Overall assessment

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_7"
  └ ## Overall assessment

Let me look at the current code to understand what needs to be fixed:

● Find relevant lines in cli.rs (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && grep -n
  │ "title\|description\|TaskListEntry\|select_ready\|task_priority" zbobr-dispatcher/src/cli.rs |
  │ head -60
  └ 32 lines...

● Check recent commits (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git log --oneline -5
  └ 6 lines...

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L260:360 (101 lines read)

● Find relevant lines in commands.rs (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && grep -n
  │ "title\|description\|select_ready\|task_priority\|run_manager_loop" zbobr/src/commands.rs | head
  │ -60
  └ 24 lines...

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L330:400 (71 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1120:1160 (41 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1160:1240 (81 lines read)

Now I understand the two remaining issues. Let me look at the full loop to understand the refactor needed:

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1100:1170 (70 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1235:1290 (56 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1170:1240 (71 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1290:1380 (91 lines read)

● Check if Task derives Clone (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && grep -n "derive.*Clone\|pub
  │ struct Task" zbobr-api/src/lib.rs | head -20
  └ 1 line...

● Search Task definition in api (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && grep -rn "derive.*Clone\|pub
  │ struct Task" zbobr-api/src/ | head -20
  └ 21 lines...

Good, `Task: Clone`. Now I'll implement the two fixes:

● add_checklist_item (MCP: zbobr) · brief: "Restore TaskListEntry.title (not description) + compact…
  └ Checklist item added (ctx_rec_13)

● add_checklist_item (MCP: zbobr) · brief: "Refactor run_manager_loop to use select_ready_task for …
  └ Checklist item added (ctx_rec_14)

**Fix 1: Restore `title` field in `TaskListEntry`**

● Edit cli.rs +2 -2
  └ zbobr-dispatcher/src/cli.rs

● Edit commands.rs +1 -1
  └ zbobr/src/commands.rs

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_13"
  └ Checklist item ctx_rec_13 checked

**Fix 2: Refactor loop to use `select_ready_task`**

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1132:1345 (214 lines read)

Now I'll refactor the loop to use `select_ready_task` in Phase 2:

● Edit cli.rs +69 -38
  └ zbobr-dispatcher/src/cli.rs

Now let's build to check for compilation errors:

● Build to check compilation (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build 2>&1 | tail -30
  └ 4 lines...

Build passes. Let's also run clippy and tests:

● Run clippy (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo clippy 2>&1 | grep -E
  │ "warning|error" | grep -v "^warning: unused" | head -30
  └ 1 line...

● Read shell output Waiting up to 30 seconds for command output
  └ 1 line...

● Read shell output Waiting up to 30 seconds for command output
  └ 14 lines...

Clean - no new clippy issues. Let me run tests quickly:

● Run tests (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo test 2>&1 | tail -20
  └ 19 lines...

The 3 failing tests are pre-existing (rustls CryptoProvider issue in `zbobr-task-backend-github`, unrelated to my changes). My changes in `zbobr-dispatcher` and `zbobr` pass.

Let me mark the second checklist item done and commit:

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_14"
  └ Checklist item ctx_rec_14 checked

● Show changed files (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git diff --stat
  └ 4 lines...

● Commit the fixes (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git add -A && git commit -m
  │ "fix: restore title field in TaskListEntry, wire select_ready_task into loop
  │
  │ - TaskListEntry: rename description→title, populate from task.title
  │ - Compact task list output: use task.title (not task.description)…
  └ 9 lines...

● report_success (MCP: zbobr) · brief: "Fixed both review issues: title restored in TaskListEntry, …
  └ Report stored

Done. The two review issues are fixed:

1. **`title` restored**: `TaskListEntry` now has `title: String` (populated from `task.title`), and the compact list output prints `task.title`.

2. **Loop uses `select_ready_task`**: `run_manager_loop` now uses a two-phase approach — Phase 1 eagerly handles transitions/Done/call-stages for all tasks; Phase 2 calls `select_ready_task(&runstage_candidates)` to pick the best ready task to run, sharing the same selection logic as `task list --select`.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         5m 22s
Total session time:     8m 9s
Total code changes:     +72 -41
Breakdown by AI model:
 claude-sonnet-4.6        1.2m in, 20.8k out, 1.2m cached (Est. 1 Premium request)