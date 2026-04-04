## Fix Applied

Rewrote `LINTER_WORKER_PROMPT` in `zbobr/src/init.rs` to address both review failures:

### Issue 1: Removed commit instruction
- Removed step 5: `"Commit the fixes with a message like \`chore: fix linting issues\`."`
- Agents must never create commits autonomously per project rules.

### Issue 2: Removed pipeline workflow details
- Removed references to "the linter stage" and "escalates to the general worker"
- Reworded intro from "Fix the formatting and linting issues reported by the linter stage" to "Fix formatting and linting issues in the codebase"
- Reworded step 1 to remove "the linter's failure report" → "any failure reports"
- Reworded step 6 to "Call `{mcp_report_failure}` with details if some issues cannot be fixed" (no mention of what happens next)
- Reworded "functional testing is handled by a separate stage" → "functional testing is out of scope for this role"

### Verification
`cargo build` succeeds with no errors.