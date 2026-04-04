● Inspect branch commits and changed files (shell)
  │ git --no-pager log --oneline origin/main..HEAD && printf '\n---DIFF---\n' && git --no-pager diff
  │ --stat origin/main...HEAD && printf '\n---FILES---\n' && git --no-pager diff --name-only
  │ origin/main...HEAD
  └ 10 lines...

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_4"
  └ ## Summary

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_5"
  └ # --select flag for `task process`

● get_ctx_rec (MCP: zbobr) · id: "ctx_rec_1"
  └ # Plan: Add `--select` flag for `task process`

I’ve got the scope narrowed to a single-file change. Next I’m checking the exact implementation against the `task list --select` analog and looking for any subtle behavior mismatches or unnecessary edits.

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L100:180 (80 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L300:500 (201 lines read)

● Show full diff for commands file (shell)
  │ git --no-pager diff origin/main...HEAD -- zbobr/src/commands.rs
  └ 44 lines...

The implementation is close to the planned analog. I’m doing one more pass for whether the new flag is enforced in the same layer as the rest of the CLI contract, especially around argument exclusivity and reuse of existing patterns.

● Search (grep)
  │ "select: bool|--select|select_runnable_task|require_task_id\(" (zbobr/src/commands.rs)
  └ 11 lines found

● Search (grep)
  │ "conflicts_with|required_unless|requires =" (zbobr/src)
  └ No matches found

● View base branch analog context (shell)
  │ git --no-pager show origin/main:zbobr/src/commands.rs | sed -n '120,160p;420,455p'
  └ 78 lines...

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L70:100 (31 lines read)

● Read commands.rs
  │ zbobr/src/commands.rs
  └ L280:295 (16 lines read)

I found one likely review point: the new behavior is wired correctly, but the exclusivity contract is still enforced at runtime rather than by Clap, unlike what the type/CLI layer could guarantee. I’m confirming that before I file it.

● Search (grep)
  │ "ArgGroup|group =|conflicts_with =|required_unless_present" (zbobr/src)
  └ No matches found

● Search (grep)
  │ "fn select_runnable_task|pub fn select_runnable_task"
  └ 8 lines found

● Check diff for obvious patch issues (shell)
  │ git --no-pager diff origin/main...HEAD --check
  └ 1 line...

I don’t see a functional mismatch with the selected-task behavior itself. The remaining question is whether the CLI contract was implemented in the right layer and whether similar command parsing paths are covered by tests.

● Search (grep)
  │ "TaskSubcommand::Process|task process|--select" in **/*{test,tests}*.rs
  └ No matches found

● Search (grep)
  │ "Process \{|List \{|TaskSubcommand" (zbobr/src)
  └ 15 lines found

● report_failure (MCP: zbobr) · brief: "Found one review issue: `task process` enforces `task` vs `…
  └ Report stored

**Review failed:** the new `task process --select` behavior is functionally close to the plan, but there’s one blocking issue.

The selection logic itself is consistent with the `task list --select` analog: it reuses `select_runnable_task`, preserves the exit-code-1 no-match behavior, and the diff is cleanly scoped to `zbobr/src/commands.rs`. The problem is that `task` vs `--select` exclusivity is enforced manually inside the handler with `anyhow::bail!` instead of declaratively in Clap. That CLI contract should live in the parser layer via `conflicts_with` or an arg group so invalid invocations are rejected during parsing and the constraint remains robust across future refactors.

--- stderr ---

Total usage est:        1 Premium request
API time spent:         1m 2s
Total session time:     1m 17s
Total code changes:     +0 -0
Breakdown by AI model:
 gpt-5.4                  196.2k in, 3.2k out, 168.2k cached (Est. 1 Premium request)