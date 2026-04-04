## Plan: Add `linter_worker` stage for fixing linter issues

### Problem
When the linter finds issues, `on_failure` currently sends the task back to the general `working` stage, which then goes through the full review/test cycle again. This is wasteful for simple formatting/linting fixes.

### Proposed Solution
Add a dedicated `linter_worker` stage that handles linter fixes in a tight loop: `linting → linter_worker → linting`.

### Analog
The `test_worker` stage/role pattern — a dedicated worker stage with its own role and prompt, focused on a specific type of fix.

### Changes

**1. Separate linter into check-only and fix roles**

- **Update `linting` stage**: Change `on_failure` from `working` to `linter_worker`
- **Update `LINTER_PROMPT`**: Remove the fixing instructions. The linter should only check and report — no auto-fixes, no commits. On failure, it reports the issues found so `linter_worker` can act on them.

**2. Add `linter_worker` stage** (in `init.rs`, in the `main_stages` IndexMap, between `linting` and `testing`):
- `role: "linter_worker"`
- `on_success` → `linting` (loop back to re-check)
- `on_failure` → `working` (escalate if linter_worker can't fix the issues)

**3. Add `linter_worker` role definition** (following `test_worker` pattern):
- MCP tools: same as worker (StopWithError, ReportSuccess, ReportFailure, StopWithQuestion, GetCtxRec — no checklist tools needed since it's a focused fixer)
- Prompt: `linter_worker.md`
- Tool: `"developer"` (needs code editing capability, unlike the linter which uses `"drudge"`)

**4. Add `LINTER_WORKER_PROMPT`**:
- Focused on fixing linter/formatting issues reported by the linter stage
- Should run the same lint/fmt commands to understand what's broken
- Apply auto-fixes and manual fixes for linter issues only
- Commit fixes, then report success (which loops back to linting for verification)
- Report failure if issues can't be resolved (escalates to general worker)
- Must NOT modify logic, tests, or functionality — only fix formatting/linting

**5. Register prompt file**: Add `("linter_worker", LINTER_WORKER_PROMPT)` to the `PROMPT_FILES` array.

### Flow comparison

**Before**: `linting` fails → `working` → `reviewing` → `test_planner` → `test_worker` → `linting` → ...
**After**: `linting` fails → `linter_worker` → `linting` (tight loop, no full pipeline re-run)

### Verification
- Run `cargo build` to ensure compilation
- Run existing tests (especially workflow/transition tests) to ensure nothing breaks
- Validate that the new stage/role passes config validation (all referenced stages and roles exist)