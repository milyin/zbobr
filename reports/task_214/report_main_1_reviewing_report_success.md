## Scope / diff inspected
- Compared `origin/main...HEAD`
- Changes limited to **`zbobr/src/init.rs`** (PLANNER_PROMPT text only). No extraneous code changes.

## Requirement coverage
- ✅ Planner is instructed to:
  - create **checklist items only when approval is confirmed**
  - call `{mcp_report_success}` **after** checklist creation
  - otherwise call `{mcp_report_intermediate}` **without** creating checklist items
- ✅ Approval sources match task intent:
  - **last (most recent) user comment**
  - or task description explicitly stating preapproval (e.g. “plan is preapproved”)

## Analog / pattern consistency
- Prompt formatting and workflow structure remain consistent with surrounding agent prompts (numbered steps, clear tool gating, no pipeline/stage details).
- Step references are consistent: Access Model points to workflow step 8; workflow contains step 8.

## Robustness / clarity notes (non-blocking suggestions)
1) **Avoid false-positive approvals**: Step 7 could explicitly say that if the last comment is “Approved”, the planner should only treat it as approval **if the plan being produced matches what was approved / there’s no meaningful deviation**; otherwise treat approval as not confirmed.
2) Consider adding one explicit sentence to incorporate “earlier plan variants and user feedback” (currently implied via reading context/comments, but not stated).

## Overall
This change satisfies the task’s core goal: it removes the prior ambiguity that could lead to reporting intermediate even after approval, and it avoids generating checklist noise when approval is pending.