● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_19"
  └ ## Overall assessment

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_17"
  └ Add `select_runnable_task(workflow: &Workflow, tasks: &[Task]) -> Option<&Tas...

● Check current branch commits (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git --no-pager log --oneline
  │ -10
  └ 11 lines...

● Find relevant functions in cli.rs (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && grep -n
  │ "apply_ready_from_state\|select_runnable_task\|run_manager_loop\|Phase 1\|Phase 2"
  │ zbobr-dispatcher/src/cli.rs | head -60
  └ 13 lines...

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L285:380 (96 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L889:960 (72 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1140:1250 (111 lines read)

● Read workflow.rs
  │ zbobr-dispatcher/src/workflow.rs
  └ L252:330 (79 lines read)

● Find State methods (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && grep -n "fn is_ready\|fn
  │ is_pause\|fn is_done\|fn is_running" zbobr-api/src/task.rs | head -20
  └ 5 lines...

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L403:430 (28 lines read)

● Find apply_pause_to_state usage (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && grep -n "apply_pause_to_state"
  │ zbobr-dispatcher/src/cli.rs | head -10
  └ 4 lines...

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L819:890 (72 lines read)

Now I understand the bug. Tasks in `READY` state with a non-empty stack are normalized by the loop (via `apply_ready_from_state`) then deferred to the next cycle — they're **never** added to `runstage_candidates`. But `select_runnable_task` calls `resolve_next_action` on them directly, yielding wrong results.

The fix: exclude `READY`-with-stack tasks from `select_runnable_task`.

● add_checklist_item (MCP: zbobr) · brief: "Fix select_runnable_task: exclude READY-with-stack task…
  └ Checklist item added (ctx_rec_20)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L297:315 (18 lines read)

● Edit cli.rs +9
  └ zbobr-dispatcher/src/cli.rs

Now build to verify:

● Build to verify changes compile (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build -p zbobr-dispatcher
  │ 2>&1 | tail -20
  └ 21 lines...

Build succeeds. Now commit and mark the checklist item done:

● Commit the fix (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git add
  │ zbobr-dispatcher/src/cli.rs && git commit -m "fix: exclude READY-with-stack tasks from
  │ select_runnable_task
  │
  │ Tasks in READY state with a non-empty stack are normalised by the…
  └ 13 lines...

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_20"
  └ Checklist item ctx_rec_20 checked

● report_success (MCP: zbobr) · brief: "Fixed: select_runnable_task now excludes READY-with-stack t…
  └ Report stored

Fixed. The issue was in `select_runnable_task`: tasks in `READY` state with a non-empty stack were being evaluated via `resolve_next_action` directly, but the loop normalizes these in Phase 1 (`apply_ready_from_state`) and defers them — they never reach Phase 2's `runstage_candidates`.

Added a one-line guard: `let ready_with_stack = t.state.is_ready() && !t.stack.is_empty()` and exclude them from the filter, making `--select` behaviorally equivalent to the loop.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 45s
Total session time:     2m 55s
Total code changes:     +9 -0
Breakdown by AI model:
 claude-sonnet-4.6        600.9k in, 5.5k out, 563.9k cached (Est. 1 Premium request)