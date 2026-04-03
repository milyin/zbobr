● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_22"
  └ ## Overall assessment

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L155:200 (46 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L200:280 (81 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L340:420 (81 lines read)

The fix is straightforward. I need to:
1. Remove `TaskSubcommand::Show { id: None, .. }` from `needs_backends()` 
2. Remove the corresponding arm from `run_without_backends()`

● add_checklist_item (MCP: zbobr) · brief: "Fix task show --json routing: remove Show{id:None} from…
  └ Checklist item added (ctx_rec_23)

● Edit commands.rs -3
  └ zbobr/src/commands.rs

● Edit commands.rs +6 -18
  └ zbobr/src/commands.rs

Now let's build to verify:

● Build zbobr to verify changes compile (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && cargo build -p zbobr 2>&1 |
  │ tail -20
  └ 4 lines...

Build succeeds. Now commit and mark the checklist item done:

● Commit the routing fix (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-240/zbobr && git add zbobr/src/commands.rs
  │ && git commit -m "fix: route task show (no ID) through backend-aware path
  │
  │ Remove TaskSubcommand::Show { id: None } from needs_backends() no-backend
  │ fast path so that 'task show' and 'task show --json' always go through…
  └ 6 lines...

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_23"
  └ Checklist item ctx_rec_23 checked

● report_success (MCP: zbobr) · brief: "Fixed task show --json routing: removed Show{id:None} from …
  └ Report stored

Fixed. The `task show --json` (no ID) was being routed to `run_without_backends()` which always returned the sample task. Removed `Show { id: None }` from the no-backend fast path — now `task show` always goes through the backend-aware path where real tasks are fetched and serialized.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 12s
Total session time:     1m 54s
Total code changes:     +6 -21
Breakdown by AI model:
 claude-sonnet-4.6        351.2k in, 2.7k out, 326.9k cached (Est. 1 Premium request)