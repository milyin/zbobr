# Plan: Prompt Fixes

## Summary

Both changes are prompt-only edits to the string constants in `zbobr/src/init.rs`. No structural Rust code changes are needed.

---

## Change 1 — Tester: Allow formatting fixes

**Problem:** Tester rejects jobs for formatting issues, sending the whole task back through the worker loop for trivial auto-fixable issues.

**Solution:** In `TESTER_PROMPT`, relax the "read-only / do not modify files" restriction specifically for auto-formatting. The tester should:
1. Run tests; if formatting checks fail, run the auto-formatter (e.g. `cargo fmt`), commit the formatting fix, then re-run the test suite.
2. Continue to reject for real test failures, but NOT for formatting that was auto-fixed.

Key prompt changes:
- Remove the absolute "read-only access" / "Do not modify files" language.
- Replace with: write access is allowed **only** to commit auto-format fixes; no logic/code changes are permitted.
- Add a new workflow step: if a formatting check fails, run the formatter, commit the fix (`git commit`), then continue testing.

---

## Change 2 — Planner: Stricter approval detection

**Problem:** The planner is treating ambiguous or non-committal user comments as approval, e.g. a comment that just asks a question or discusses the plan.

**Solution:** In `PLANNER_PROMPT` step 7, tighten the approval criteria with explicit examples:
- Approval requires an **unambiguous positive signal** such as: "approved", "yes", "go ahead", "proceed", "LGTM", "+1", "looks good", or the task description explicitly saying the plan is pre-approved.
- NOT approval: general discussion, partial feedback, questions, acknowledgement of the plan without explicit sign-off, silence.
- Default to NOT approved when in doubt.

---

## Analog

Both changes modify existing string constants in `zbobr/src/init.rs`. No new structures or files are needed.
