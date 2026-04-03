● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_7"
  └ ## Overall assessment

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_6"
  └ # Implementation Complete

● Inspect relevant parts of cli.rs (shell)
  │ cat /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs |
  │ grep -n "TaskListEntry\|select_ready_task\|is_ready\|is_pause\|is_done\|stage_count\|state\|titl
  │ e\|description" | head -60
  └ 61 lines...

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_2"
  └ ## What

● View the loop task selection logic (shell)
  │ sed -n '1090,1130p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs
  └ 42 lines...

● Find loop and select_ready_task usage (shell)
  │ grep -n
  │ "select_ready_task\|run_manager_loop\|is_ready\|is_pause\|stage_count\|skip\|max_by_key\|sort"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -60
  └ 31 lines...

● View more of the loop logic (shell)
  │ sed -n '1130,1175p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs
  └ 47 lines...

● Find State type and its methods (shell)
  │ grep -n "is_ready\|is_done\|is_pause\|State::"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-api/src/lib.rs | head -40
  └ 1 line...

● Find State enum definition and methods (shell)
  │ grep -rn "fn is_ready\|fn is_done\|fn is_pause\|pub enum State\|State {"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-api/src/ | head -30
  └ 13 lines...

● Check commands.rs for current implementation (shell)
  │ cat /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs | grep -n
  │ "TaskListEntry\|select_ready_task\|json\|--json\|--select\|Show\|List\|task show\|task list" |
  │ head -50
  └ 27 lines...

● View full commands.rs task list/show logic (shell)
  │ sed -n '260,400p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs
  └ 142 lines...

● View State enum and methods (shell)
  │ sed -n '377,415p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-api/src/task.rs
  └ 40 lines...

● Check for is_running method (shell)
  │ grep -n "fn is_running\|is_running"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-api/src/task.rs | head -20
  └ 6 lines...

● View TaskListEntry struct and select_ready_task in cli.rs (shell)
  │ sed -n '260,300p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs
  └ 41 lines...

● View loop behavior after processing tasks (shell)
  │ sed -n '1175,1220p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs
  └ 47 lines...

● View more loop code (shell)
  │ sed -n '1220,1260p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs
  └ 42 lines...

● View Done/StateAction handling in loop (shell)
  │ sed -n '1226,1310p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs
  └ 86 lines...

● View start of run_manager_loop (shell)
  │ sed -n '1051,1095p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs
  └ 46 lines...

● Find session_run and break points in loop (shell)
  │ grep -n "session_run\|break\|continue\|StateAction"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -30
  └ 25 lines...

Now I have a clear picture. Three fixes needed:
1. `select_ready_task` should exclude running tasks + be wired into loop via shared priority function
2. `task show --json` no-ID path should serialize `Vec<Task>` not `Vec<TaskListEntry>`
3. `TaskListEntry` should use `description` not `title`

● add_checklist_item (MCP: zbobr) · brief: "Fix select_ready_task: exclude Running tasks, extract s…
  └ Checklist item added (ctx_rec_8)

● add_checklist_item (MCP: zbobr) · brief: "TaskListEntry: rename title→description, update From im…
  └ Checklist item added (ctx_rec_9)

● add_checklist_item (MCP: zbobr) · brief: "Fix task show --json no-ID path: serialize Vec<Task> no…
  └ Checklist item added (ctx_rec_10)

Now let me implement all three fixes:

● Edit cli.rs +17 -6
  └ zbobr-dispatcher/src/cli.rs

Now update `run_manager_loop`'s sort to use `task_priority`:

● Edit cli.rs +2 -2
  └ zbobr-dispatcher/src/cli.rs

Now fix the `commands.rs` issues:

● View compact list display line (shell)
  │ sed -n '355,360p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs
  └ 7 lines...

● Edit commands.rs +1 -1
  └ zbobr/src/commands.rs

Now fix `task show --json` no-ID path to serialize `Vec<Task>`:

● View show no-ID json path (shell)
  │ sed -n '370,390p'
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs
  └ 22 lines...

● Edit commands.rs +1 -2
  └ zbobr/src/commands.rs

Now check if `TaskListEntry` is still used in `commands.rs` (to see if the import needs updating):

● Check TaskListEntry usage in commands.rs (shell)
  │ grep -n "TaskListEntry"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs
  └ 3 lines...

Good, `TaskListEntry` is still used in the `List` JSON path at line 350. Now let's build to check for errors:

● Build to verify changes compile (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build -p zbobr-dispatcher
  │ -p zbobr 2>&1
  └ 6 lines...

Clean build. Now run the tests:

● Run tests (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo test -p zbobr-dispatcher
  │ -p zbobr 2>&1
  └ 138 lines...

All 80 + 14 tests pass. Let me mark the checklist items done and commit:

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_8"
  └ Checklist item ctx_rec_8 checked

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_9"
  └ Checklist item ctx_rec_9 checked

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_10"
  └ Checklist item ctx_rec_10 checked

● Check changes summary (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git --no-pager diff --stat
  └ 4 lines...

● Review full diff (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git --no-pager diff
  └ 88 lines...

The diff looks exactly right. Let me commit:

● Commit fix (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git add
  │ zbobr-dispatcher/src/cli.rs zbobr/src/commands.rs && git commit -m "fix: address review issues
  │ in task list/show/select
  │
  │ - TaskListEntry: rename title→description to match task spec…
  └ 10 lines...

● report_success (MCP: zbobr) · brief: "Fixed all three review issues: description field, select_re…
  └ Report stored

All three review issues are fixed:

1. **`description` not `title`** — `TaskListEntry` now has a `description` field, and the compact list output shows `task.description`
2. **Shared priority + Running excluded** — extracted `task_priority()` function used by both `select_ready_task` and `run_manager_loop`'s sort; `select_ready_task` now also filters out `State::Running` tasks
3. **`task show --json` (no ID)** — now serializes `Vec<Task>` with all fields, not the compact `Vec<TaskListEntry>`

--- stderr ---

Total usage est:        1 Premium request
API time spent:         3m 59s
Total session time:     5m 23s
Total code changes:     +21 -11
Breakdown by AI model:
 claude-sonnet-4.6        1.2m in, 12.3k out, 1.2m cached (Est. 1 Premium request)