I’m pulling the plan context and the full branch diff first, then I’ll inspect the implementation against the chosen analog and checklist.

● Show full task diff (shell)
  │ git --no-pager diff origin/main...HEAD
  └ 194 lines...

● Inspect commits and changed files (shell)
  │ git --no-pager log --oneline origin/main..HEAD && printf '\n---DIFF---\n' && git --no-pager diff
  │ --stat origin/main...HEAD && printf '\n---FILES---\n' && git --no-pager diff --name-only
  │ origin/main...HEAD
  └ 14 lines...

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_1"
  └ # Plan: Update task list, add JSON output

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_5"
  └ # Plan: Update task list, add JSON output

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_6"
  └ # Implementation Complete

I found the main implementation quickly; now I’m spot-checking the actual selection and display paths to see whether the shared helper is really wired through and whether `show --json` preserves full-task semantics.

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L70:410 (341 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1060:1135 (76 lines read)

● Search (grep)
  │ "select_ready_task|run_manager_loop|print_task\(|TaskSubcommand::Show|TaskSubcommand::List" in
  │ {zbobr-dispatcher/src/*.rs,zbobr/src/*.rs}
  └ 14 lines found

● Search (grep)
  │ "impl (Display|serde::Serialize).*State|enum State|is_pause\(|is_done\(" in zbobr-api/src/*.rs
  └ 4 lines found

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L377:530 (154 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L285:360 (76 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1120:1215 (96 lines read)

I found one clear mismatch already: the manager loop still keeps its own ordering/scan instead of using the new shared selector, so the “single source of truth” part of the plan wasn’t actually finished. I’m checking whether that also causes a behavioral mismatch for `--select` versus the loop, plus whether `show --json` dropped fields in the no-ID path.

● Search (grep)
  │ "session_run|break;|sleep" (zbobr-dispatcher/src/cli.rs)
  └ 9 lines found

● Search (grep)
  │ "derive\(.*Serialize.*\).*Task|pub struct Task" in zbobr-api/src/*.rs
  └ 3 lines found

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1215:1295 (81 lines read)

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L840:930 (91 lines read)

I’ve narrowed it to a couple of likely review findings, but I want one more quick pass on the task model so I don’t over-report: the implementation may also have swapped the requested `description` field for `title` in the new list entry type.

● Search (grep)
  │ "Created task|Title:|Description:|task list" (README.md)
  └ 2 lines found

● Search (grep)
  │ "TaskListEntry|title: task.title|description" in
  │ {README.md,docs/**/*.md,zbobr-api/src/*.rs,zbobr/src/*.rs,zbobr-dispatcher/src/*.rs}
  └ 71 lines found

● Search (grep)
  │ "pub struct TaskListEntry|title: String|description: String" in zbobr-*/src/*.rs
  └ 12 lines found

I’ve finished the review. There are a few substantive mismatches with the task requirements, so I’m filing this as changes-needed rather than checking it off.

● report_failure (MCP: zbobr) · brief: "Review found three issues: selector logic is not actually s…
  └ Report stored

**Review result:** changes needed.

I found three material issues: the new selector can return non-ready tasks and still isn’t used by `run_manager_loop`, `task show --json` without an ID returns compact list entries instead of full task JSON, and the new list entry/output uses `title` where the task asked for `description`. The changes are otherwise well-scoped, but I did not mark any additional checklist items complete because the shared-selection and output-contract parts are not fully satisfied yet.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 42s
Total session time:     1m 56s
Total code changes:     +0 -0
Breakdown by AI model:
 gpt-5.4                  247.6k in, 5.1k out, 211.6k cached (Est. 1 Premium request)