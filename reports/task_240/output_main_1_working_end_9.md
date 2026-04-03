● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_25"
  └ ## Overall assessment

● add_checklist_item (MCP: zbobr) · brief: "Add deterministic tie-breaker to select_runnable_task: …
  └ Checklist item added (ctx_rec_26)

● Find selector and sort logic in cli.rs (shell)
  │ grep -n "select_runnable_task\|task_priority\|max_by_key\|sort_by"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr-dispatcher/src/cli.rs | head
  │ -60
  └ 9 lines...

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L285:335 (51 lines read)

● Find selector usage in commands.rs (shell)
  │ grep -n "select_runnable_task\|task_priority\|max_by_key\|sort_by_key\|sort_by"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr/zbobr/src/commands.rs | head -30
  └ 5 lines...

Now I'll fix `select_runnable_task` to use a total ordering with a deterministic tie-breaker (task ID descending — so highest ID wins ties, which is consistent and deterministic):

● Edit cli.rs +5 -1
  └ zbobr-dispatcher/src/cli.rs

Now verify the change and build:

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L308:326 (19 lines read)

● Build to verify no compile errors (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build --quiet 2>&1 | head
  │ -30
  └ 1 line...

Build succeeds. Now commit and mark the checklist item done:

● Commit tie-breaker fix (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git add
  │ zbobr-dispatcher/src/cli.rs && git commit -m "fix: add deterministic tie-breaker to
  │ select_runnable_task
  │
  │ Use (stage_count DESC, task_id DESC) total ordering so that --select…
  └ 7 lines...

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_26"
  └ Checklist item ctx_rec_26 checked

● report_success (MCP: zbobr) · brief: "Added deterministic tie-breaker to select_runnable_task: no…
  └ Report stored

Fixed. The `select_runnable_task` function now uses a total ordering `(stage_count DESC, task_id DESC)` as a tie-breaker, ensuring `task list --select` and `run_manager_loop` always choose the same highest-priority runnable task regardless of how the input slice was pre-sorted.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 15s
Total session time:     1m 56s
Total code changes:     +5 -1
Breakdown by AI model:
 claude-sonnet-4.6        338.0k in, 2.6k out, 315.4k cached (Est. 1 Premium request)