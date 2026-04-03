● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_16"
  └ ## Overall assessment

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_12"
  └ ## Overall assessment

● Find select_ready_task and related code (shell)
  │ grep -n "select_ready_task\|resolve_next_action\|runstage_candidates\|StateAction\|task_priority
  │ \|RunStage"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -60
  └ 26 lines...

● Find workflow resolve_next_action (shell)
  │ grep -n "select_ready_task\|resolve_next_action\|runstage_candidates\|StateAction\|task_priority
  │ \|RunStage"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/workflow.rs |
  │ head -60
  └ 32 lines...

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L285:360 (76 lines read)

● Find --select usage in commands.rs (shell)
  │ grep -n "\-\-select\|select_ready\|TaskListEntry\|task list\|SubCommand\|List\b"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs | head -60
  └ 8 lines...

● Find --select usage in cli.rs (shell)
  │ grep -n "\-\-select\|select_ready\|TaskListEntry\|task list\|SubCommand\|List\b\|TaskList"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -60
  └ 8 lines...

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L320:365 (46 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L260:315 (56 lines read)

Now I need to understand how the zbobr CLI accesses the dispatcher/workflow to know if we can use `resolve_next_action` in the `--select` path.

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L1:60 (60 lines read)

● Find dispatcher usage in commands.rs (shell)
  │ grep -n "zbobr_dispatcher\|ZbobrDispatcher\|workflow\(\)"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs | head -30
  └ 25 lines...

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L303:345 (43 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1125:1345 (221 lines read)

● Check lib.rs exports (shell)
  │ grep -n "select_ready_task\|select_runnable_task\|pub use\|pub fn select"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/lib.rs | head
  │ -30
  └ 13 lines...

● Read lib.rs
  │ zbobr-dispatcher/src/lib.rs
  └ L20:40 (21 lines read)

Now I have a complete picture. The fix: add `select_runnable_task(workflow, tasks)` that uses `resolve_next_action` to filter (matching the loop's actual predicate), and use it in both `--select` and the loop's Phase 2.

● add_checklist_item (MCP: zbobr) · brief: "Add select_runnable_task using resolve_next_action, use…
  └ Checklist item added (ctx_rec_17)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L285:312 (28 lines read)

● Edit cli.rs +12 -7
  └ zbobr-dispatcher/src/cli.rs

Now update loop Phase 2 to use `select_runnable_task`:

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1320:1340 (21 lines read)

● Edit cli.rs +2 -2
  └ zbobr-dispatcher/src/cli.rs

Now update lib.rs to export `select_runnable_task` instead of `select_ready_task`:

● Edit lib.rs +1 -1
  └ zbobr-dispatcher/src/lib.rs

Now update `commands.rs` to use `select_runnable_task` instead of `select_ready_task`:

● Edit commands.rs +1 -1
  └ zbobr/src/commands.rs

Now update the `--select` call site in `commands.rs`. I need to pass the workflow:

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L339:350 (12 lines read)

● Edit commands.rs +1 -1
  └ zbobr/src/commands.rs

Now I need to check if `zbobr.workflow()` is accessible in this context and what it returns. Let me check:

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L300:315 (16 lines read)

Good - `zbobr` is available. Let me check what `zbobr.workflow()` returns:

● Check workflow() return type (shell)
  │ grep -n "fn workflow\(\)"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -10
  └ 1 line...

● Find workflow accessor (shell)
  │ grep -n "pub fn workflow"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/*.rs | head -10
  └ 3 lines...

`zbobr.workflow()` returns `&Workflow`. Now let's check the signature of `select_runnable_task`:

The function signature is `pub fn select_runnable_task<'a>(workflow: &Workflow, tasks: &'a [Task]) -> Option<&'a Task>`. But in the loop, the workflow is not `zbobr.workflow()` - let me check what type `workflow` is in the loop:

● Find workflow usage in loop (shell)
  │ grep -n "let workflow\b\|workflow ="
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -20
  └ 2 lines...

Good, `workflow` in the loop is `&Workflow` from `zbobr.workflow()`. Now let me build to check for any issues:

● Build to check for errors (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build 2>&1 | tail -30
  └ 15 lines...

● Check workflow type usage in cli.rs (shell)
  │ grep -n "use crate::workflow\|use super::workflow\|workflow::Workflow\|Workflow"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -10
  └ 3 lines...

● Check imports in cli.rs (shell)
  │ grep -n "^use\|^pub use"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -20
  └ 9 lines...

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L21:27 (6 lines read)

● Edit cli.rs +1 -1
  └ zbobr-dispatcher/src/cli.rs

● Build again (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build 2>&1 | tail -30
  └ 4 lines...

Build succeeds. Let me also check for any unused import warnings:

● Check for warnings (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build 2>&1 | grep -E
  │ "warning|unused"
  └ 1 line...

● Full build output (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build 2>&1
  └ 2 lines...

Clean build with no warnings. Let me also run tests:

● Run tests (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo test 2>&1 | tail -20
  └ 19 lines...

● Check baseline test failures (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git stash && cargo test 2>&1 |
  │ tail -10
  └ 12 lines...

Pre-existing failures. Let me restore my changes:

● Restore changes (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git stash pop
  └ 14 lines...

The test failures are pre-existing. My changes are clean. Let me verify the final state of the key changes:

● Show full diff of changes (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git diff
  └ 81 lines...

● Read shell output Waiting up to 5 seconds for command output
  └ 81 lines...

● Stop shell
  └ 1 line...

The diff looks correct. Now let me commit:

● Commit the fix (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git add
  │ zbobr-dispatcher/src/cli.rs zbobr-dispatcher/src/lib.rs zbobr/src/commands.rs && git commit -m
  │ "fix: replace select_ready_task with workflow-resolved select_runnable_task
  │
  │ Replaced the simple state-flag predicate with a full workflow resolution…
  └ 12 lines...

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_17"
  └ Checklist item ctx_rec_17 checked

● report_success (MCP: zbobr) · brief: "Fixed: replaced select_ready_task with select_runnable_task…
  └ Report stored

The fix is complete. 

**What was wrong:** `select_ready_task` used simple state-flag checks (`!is_done() && !pause && !is_pause() && !is_running()`), which is weaker than the loop's actual predicate that uses `workflow.resolve_next_action()` to confirm a task resolves to a non-call `RunStage` action.

**What was changed:**
- Replaced `select_ready_task` with `select_runnable_task(workflow, tasks)` that calls `workflow.resolve_next_action()` and keeps only non-call `RunStage` tasks
- Both `task list --select` and the loop Phase 2 now use the same function, ensuring they agree on which task is next

--- stderr ---

Total usage est:        1 Premium request
API time spent:         4m 25s
Total session time:     6m 32s
Total code changes:     +18 -13
Breakdown by AI model:
 claude-sonnet-4.6        1.8m in, 14.4k out, 1.7m cached (Est. 1 Premium request)