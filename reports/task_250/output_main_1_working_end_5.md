● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_11"
  └ The reviewer found that the new linting workflow wiring is implemented but no...

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_13"
  └ The implementation is otherwise focused and consistent with the chosen analog...

● Find drudge and linting references in init.rs (shell)
  │ grep -n "drudge\|linting\|linter"
  │ /data/home/skynet/zdam/zbobr-dev/workspaces/task-250/zbobr/zbobr/src/init.rs | head -60
  └ 45 lines...

● add_checklist_item (MCP: zbobr) · brief: "Strengthen drudge tool tests to verify exact model/prio…
  └ Checklist item added (ctx_rec_14)

● Read init.rs
  │ zbobr/src/init.rs
  └ L195:225 (31 lines read)

● Read init.rs
  │ zbobr/src/init.rs
  └ L1220:1260 (20 lines read)

● Search (grep)
  │ "struct ToolEntry"
  └ 1 line found

● Search (grep)
  │ "struct ToolEntry"" (zbobr-api/src/config.rs)
  └ No matches found

● Search (grep)
  │ "ToolEntry" (zbobr-api/src/config.rs)
  └ 138 lines found

Now I have what I need. Let me strengthen the `default_config_toml_has_drudge_tool` test to verify the exact model/provider/priority contract:

● Edit init.rs +31 -3
  └ zbobr/src/init.rs

Now let me check what `Model::to_string()` returns for these models to make sure the assertions will pass:

● Search (grep)
  │ "impl.*Display.*for Model|fn fmt.*Model|gpt-5-mini|haiku" in **/*.rs
  └ 21 lines found

● Search (grep)
  │ "impl.*Display.*for Model" (zbobr-api/src/task.rs)
  └ 6 lines found

Good - `Model::to_string()` returns the inner string, so `"gpt-5-mini".parse().unwrap().to_string()` will equal `"gpt-5-mini"`. Now let me run the tests:

● Run init tests (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-250/zbobr && cargo test -p zbobr --lib --
  │ init::tests 2>&1 | tail -30
  └ 2 lines...

● Run init tests (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-250/zbobr && cargo test -p zbobr --
  │ init::tests 2>&1 | tail -40
  └ 23 lines...

All 15 tests pass. Now let me commit and mark the checklist item done:

● Commit strengthened drudge tests (shell)
  │ cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-250/zbobr && git add zbobr/src/init.rs &&
  │ git commit -m "test: verify drudge tool model/priority contract in init tests
  │
  │ Assert exact provider, model and priority values for both entries
  │ of the drudge tool (copilot/gpt-5-mini primary, claude/claude-haiku-4.5…
  └ 6 lines...

● check_checklist_item (MCP: zbobr) · id: "ctx_rec_14"
  └ Checklist item ctx_rec_14 checked

● report_success (MCP: zbobr) · brief: "Strengthened drudge tool tests to verify exact model/priori…
  └ Report stored

Done. Strengthened the `default_config_toml_has_drudge_tool` test to assert the exact `drudge` tool contract — 2 entries with precise provider/model/priority values. All 15 tests pass.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 53s
Total session time:     2m 22s
Total code changes:     +31 -3
Breakdown by AI model:
 claude-sonnet-4.6        504.7k in, 3.7k out, 471.6k cached (Est. 1 Premium request)