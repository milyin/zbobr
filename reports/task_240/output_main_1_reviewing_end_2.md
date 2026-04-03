I’m pulling the task history and full branch diff first, then I’ll spot-check the changed code against the planned analog and checklist.

● Inspect branch commits and changed files (shell)
  │ git --no-pager log --oneline origin/main..HEAD && printf '\n---DIFF---\n' && git --no-pager diff
  │ --stat origin/main...HEAD && printf '\n---NAMES---\n' && git --no-pager diff --name-only
  │ origin/main...HEAD
  └ 17 lines...

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_5"
  └ # Plan: Update task list, add JSON output

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_15"
  └ ## Summary

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_12"
  └ ## Overall assessment

I have the planner and prior review context; now I’m reading the actual changed code to verify the final state rather than trusting earlier reports.

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L300:420 (121 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1:420 (420 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1080:1285 (206 lines read)

● Read lib.rs
  │ zbobr-dispatcher/src/lib.rs
  └ L1:120 (119 lines read)

The main behavior is in place, so I’m checking the CLI surface and whether the shared selector is actually reused consistently, including the new flags and output contracts.

● Search (grep)
  │ "TaskSubcommand::List|TaskSubcommand::Show|--json|--select|json: bool|select: bool"
  │ (zbobr/src/commands.rs)
  └ 50 lines found

● Search (grep)
  │ "select_ready_task\(|task_priority\("
  └ 35 lines found

I found the selector call sites; I’m checking the surrounding control flow and the CLI definitions to make sure there isn’t a hidden mismatch in behavior or help surface.

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L60:110 (51 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L240:290 (51 lines read)

● Read cli.rs
  │ zbobr-dispatcher/src/cli.rs
  └ L1318:1395 (78 lines read)

The core fixes look present, so I’m doing one more pass for contract mismatches and subtle review-worthy issues in the changed paths only.

● Search (grep)
  │ "impl std::fmt::Display for State|derive\(.*Serialize.*\).*enum State|pub enum State|impl State"
  │ (zbobr-api)
  └ 17 lines found

● Read final patch for changed files (shell)
  │ git --no-pager diff origin/main...HEAD -- zbobr-dispatcher/src/cli.rs zbobr/src/commands.rs
  │ zbobr-dispatcher/src/lib.rs | sed -n '1,260p'
  └ 261 lines...

I’m checking one last subtle point: whether “ready” in the shared selector really matches the scheduler’s notion of runnable tasks, not just “not paused/running/done.”

● Read task.rs
  │ zbobr-api/src/task.rs
  └ L377:470 (94 lines read)

● Search (grep)
  │ "enum StateAction|StateAction::Idle|StateAction::Paused|resolve_next_action\("
  │ (zbobr-dispatcher/src)
  └ 115 lines found

I found a potential semantic mismatch in `--select`; I’m pulling the state-machine branch that defines “idle” so the review can cite the exact divergence from runnable-task behavior.

● Read workflow.rs
  │ zbobr-dispatcher/src/workflow.rs
  └ L248:315 (68 lines read)

I found one remaining correctness issue in the final branch: `task list --select` can still return an idle, non-runnable task, so the “shared lookup” requirement is not fully met in behavior.

● report_failure (MCP: zbobr) · brief: "Review failed: `task list --select` still uses a weaker rea…
  └ Report stored

**Review failed:** `task list --select` still uses a broader readiness check than the loop, so it can print an ID for a task the scheduler would actually treat as idle.

The remaining issue is in the shared-selection behavior:
- `task list --select` calls `select_ready_task(&tasks)` on the full task list.
- `run_manager_loop()` only calls `select_ready_task()` **after** filtering tasks through `workflow.resolve_next_action()` and keeping only `RunStage(...)` candidates.
- Because of that mismatch, states like `Pending(...)` without a signal or `Ready` with a stack but no signal can be selected by the CLI even though the loop would skip them as `Idle`.

The analog choice and the rest of the changes look consistent, and I didn’t find unrelated churn.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 26s
Total session time:     1m 40s
Total code changes:     +0 -0
Breakdown by AI model:
 gpt-5.4                  312.9k in, 4.9k out, 285.4k cached (Est. 1 Premium request)