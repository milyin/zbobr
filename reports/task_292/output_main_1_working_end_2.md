● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_5"
  └ # --select flag for `task process`

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_6"
  └ Review result: changes are narrowly scoped to `zbobr/src/commands.rs` and mos...

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L120:160 (41 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L430:465 (36 lines read)

Now let me look at how the `List` command handles its `select` flag to understand the analog:

● Search (grep)
  │ "select" (zbobr/src/commands.rs)
  └ 11 lines found

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L80:100 (21 lines read)

The fix is to use Clap's `conflicts_with` declaratively instead of the runtime `bail!` check. Let me apply that:

● add_checklist_item (MCP: zbobr) · brief: "Fix: enforce `task` vs `--select` exclusivity in Clap p…
  └ Checklist item added (ctx_rec_7)

● Edit commands.rs +1
  └ zbobr/src/commands.rs

● Edit commands.rs -3
  └ zbobr/src/commands.rs

● Build zbobr to verify changes compile (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-292/zbobr && cargo build -p zbobr 2>&1 |
  │ tail -20
  └ 5 lines...

Build succeeds. Let me verify the final state of the relevant section:

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L433:453 (21 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L129:138 (10 lines read)

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_7"
  └ Checklist item ctx_rec_7 checked

● report_success (MCP: zbobr) · brief: "Fixed parser-level exclusivity for `task process --select` …
  └ Report stored

Fixed. Changed from a runtime `anyhow::bail!` check to a declarative `#[arg(conflicts_with = "select")]` on the positional `task` argument, so Clap rejects `zbobr task process <ID> --select` during parsing with a proper CLI error.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 2s
Total session time:     1m 36s
Total code changes:     +1 -3
Breakdown by AI model:
 claude-sonnet-4.6        381.8k in, 2.8k out, 311.2k cached (Est. 1 Premium request)