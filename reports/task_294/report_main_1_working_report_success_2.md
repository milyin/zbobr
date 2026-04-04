## Summary

Fixed two policy violations in `LINTER_WORKER_PROMPT` in `zbobr/src/init.rs`:

### Changes made

1. **Removed autonomous commit instruction** — deleted step 5 (`Commit the fixes with a message like 'chore: fix linting issues'`), which violated the project rule that agents must never create commits on their own initiative.

2. **Removed pipeline workflow details** — rewrote lines that referenced:
   - "reported by the linter stage" → "in the code"
   - "the linter's failure report" → "failure reports"
   - "the linter stage will re-verify" → removed
   - "escalates to the general worker" → removed
   - "handled by a separate stage" → "handled separately"

3. **Renumbered steps** — from 7 steps to 6 after removing the commit step.

### Build status
`cargo build` passes cleanly.

### Commit
`9d69b67e` — fix(linter_worker): remove commit instruction and pipeline stage references from prompt