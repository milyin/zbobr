## Plan: Add `linter_worker` Stage for Fixing Linter Issues

### Approach
Add a dedicated `linter_worker` stage that creates a tight fix loop: `linting → linter_worker → linting`. The linter becomes check-only; the linter_worker handles fixes.

### Key Design Decisions
- **Analog**: `test_worker` stage/role pattern — a focused worker stage that loops back to re-verify
- **Separation of concerns**: Linter checks only, linter_worker fixes only
- **Tool choice**: `developer` tool for linter_worker (needs code editing); linter keeps `drudge`
- **Escalation**: `linter_worker` on_failure → `working` (same as other worker stages when they can't resolve issues)

### Changes Summary
1. Update `linting` stage: `on_failure` → `linter_worker` (was `working`)
2. Add `linter_worker` stage between `linting` and `testing`
3. Add `linter_worker` role definition (developer tool, minimal MCP tools)
4. Refactor `LINTER_PROMPT` to be check-only (remove auto-fix logic)
5. Add `LINTER_WORKER_PROMPT` and register in `PROMPT_FILES`
6. Build verification

All changes are in `zbobr/src/init.rs`.