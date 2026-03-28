The plan is presented for your review. Here's the summary:

**Two prompt-only changes to `TESTER_PROMPT` and `PLANNER_PROMPT` constants in `zbobr/src/init.rs`:**

1. **Tester** — relax the "read-only/do not modify files" restriction to allow running auto-formatters (e.g. `cargo fmt`) and committing the result. If only formatting fails, fix+commit and continue — don't send the whole task back through the loop.

2. **Planner** — tighten step 7 approval detection. Require an explicit unambiguous positive signal ("approved", "yes", "go ahead", "LGTM", etc.). Add negative examples (general discussion, questions, partial feedback = NOT approval). Default to NOT approved when in doubt.

No structural Rust code changes needed. Please confirm to proceed.