# Fix: Remove low-level details from planner checklist item instructions

## Problem
The reviewer identified that PLANNER_PROMPT step 7 still instructed the planner to put "file paths, code snippets, specific changes, and rationale in full_report" — directly contradicting the architecture-level planning requirement introduced in the previous session.

## Change Made
**File:** `zbobr/src/init.rs` — `PLANNER_PROMPT` step 7

**Before:**
> Put concise step title in brief; put file paths, code snippets, specific changes, and rationale in full_report.

**After:**
> Put concise step title in brief; put the *what* and *why* in full_report — which components or modules to change, which interfaces or data flows are affected, which patterns from the analog to follow. Do NOT include code snippets, exact file paths, or prescriptive implementation details — the worker will look those up.

## Verification
- `cargo check` passes cleanly
- Committed as `1e4fb9c`
