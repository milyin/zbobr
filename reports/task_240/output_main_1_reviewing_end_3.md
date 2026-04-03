I’m reviewing the branch against `main` and pulling the task context first so I can check the implementation against the actual plan and prior review findings.

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_18"
  └ ## Fix applied

● Inspect branch commits and changed files (shell)
  │ git --no-pager log --oneline origin/main..HEAD && printf '\n---DIFF---\n' && git --no-pager diff
  │ --stat origin/main...HEAD && printf '\n---FILES---\n' && git --no-pager diff --name-only
  │ origin/main...HEAD
  └ 18 lines...

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_12"
  └ ## Overall assessment

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_5"
  └ # Plan: Update task list, add JSON output

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_15"
  └ ## Summary

I’ve got the branch scope and prior review trail. Next I’m reading the actual diff in the touched files to verify the shared selector, output contracts, and whether anything drifted from the plan.

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ 1 line read

● Read commands.rs
  │ zbobr/src/commands.rs
  └ 1 line read

● Read lib.rs
  │ zbobr-dispatcher/src/lib.rs
  └ 1 line read

● Show full diff for changed files (shell)
  │ git --no-pager diff origin/main...HEAD -- zbobr-dispatcher/src/cli.rs
  │ zbobr-dispatcher/src/lib.rs zbobr/src/commands.rs
  └ 361 lines...

The main risk area now is whether the extracted selector really matches the loop’s runnable-task semantics, not just the sorting. I’m checking the surrounding workflow code and the updated command handlers for subtle behavior drift.

● Search (grep)
  │ "TaskSubcommand::List|TaskSubcommand::Show|print_task\(" (zbobr/src/commands.rs)
  └ 53 lines found

● Search (grep)
  │ "TaskListEntry" in **/*.rs
  └ 29 lines found

● Search (grep)
  │ "enum StateAction|resolve_next_action|call_pipeline\(|task_priority\(|select_runnable_task\(" in
  │ **/*.rs
  └ 159 lines found

I’ve narrowed this to behavior details now: output shape, selector semantics, and whether the new helper matches the old scheduling behavior rather than just approximating it.

● Search (grep)
  │ "impl std::fmt::Display for State|enum State" in **/*.rs
  └ 19 lines found

● Search (grep)
  │ "list_tasks\(|get_task\(|assigned" in **/*.rs
  └ 8 lines found

● Inspect base branch analogs for task list and loop (shell)
  │ git --no-pager show origin/main:zbobr/src/commands.rs | sed -n '300,390p' && printf
  │ '\n---CLI---\n' && git --no-pager show origin/main:zbobr-dispatcher/src/cli.rs | sed -n
  │ '1120,1260p'
  └ 235 lines...

I found the base loop analog. Now I’m comparing the exact control flow around state transitions and tie behavior so I can tell whether the shared helper is truly equivalent or only superficially reused.

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L323:386 (64 lines read)

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L377:470 (94 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L297:314 (18 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1137:1358 (222 lines read)

I have the critical loop section now. I’m doing one last pass for output contracts and any hidden review findings like tie-breaking changes or mismatched serialization semantics.

● Search (grep)
  │ "impl serde::Serialize for State|impl std::fmt::Display for State|impl fmt::Display for State"
  │ (zbobr-api/src/task.rs)
  └ 7 lines found

● Search (grep)
  │ "No tasks found|Output as JSON array|Print the ID of the highest-priority ready task"
  │ (zbobr/src/commands.rs)
  └ 20 lines found

● Inspect diff context around helper and loop (shell)
  │ git --no-pager diff origin/main...HEAD --unified=5 -- zbobr-dispatcher/src/cli.rs | sed -n
  │ '1,260p'
  └ 243 lines...

I think the remaining question is whether `--select` sees the same pre-resolution state normalization as the loop. I’m reading the workflow and helper paths for `pause`/`ready` handling before I decide whether this is clean or still subtly inconsistent.

● Read workflow.rs
  │ zbobr-dispatcher/src/workflow.rs
  └ L205:330 (126 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L860:940 (81 lines read)

● Search (grep)
  │ "fn apply_ready_from_state|fn apply_pause_to_state|READY with
  │ stack|resolve_inner\(|State::Ready|stack" in **/*.rs (zbobr-dispatcher/src)
  └ 8 lines found

I found a likely correctness gap around `READY` tasks with resume stack: the loop normalizes them before scheduling, but `task list --select` calls the helper on raw snapshots. I’m confirming that against the workflow’s signal resolution.

● Search (grep)
  │ "fn pipeline_from_state|pipeline_from_state\(" (zbobr-dispatcher/src/workflow.rs)
  └ 15 lines found

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L917:980 (64 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L879:915 (37 lines read)

I found one remaining correctness issue. Everything else I checked is scoped to the task and mostly follows the planned analog, but the shared-selector requirement is still not fully met in behavior.

● report_failure (MCP: zbobr) · brief: "Review failed: `task list --select` can still disagree with…
  └ Report stored

Found a remaining correctness issue: **`task list --select` can still disagree with the loop** for tasks in `READY` state with a non-empty resume stack.

The loop normalizes those tasks first via `apply_ready_from_state()` before scheduling, but `select_runnable_task()` calls `workflow.resolve_next_action()` on the raw snapshot. That means `--select` can resolve a resumed task against the **default pipeline** instead of the saved pipeline from the stack, and print an ID the loop would not actually run in that cycle. I reported this as a failure; the rest of the branch looked scoped and the `title`/JSON fixes were correct.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 57s
Total session time:     2m 14s
Total code changes:     +0 -0
Breakdown by AI model:
 gpt-5.4                  380.9k in, 6.2k out, 348.9k cached (Est. 1 Premium request)