## Summary

Implemented the `linter_worker` stage in `zbobr/src/init.rs`.

### Changes made

1. **`linting` stage** — changed `on_failure` from `"working"` to `"linter_worker"`
2. **New `linter_worker` stage** — added between `linting` and `testing`:
   - `role: "linter_worker"`
   - `on_success → "linting"` (tight loop for re-verification)
   - `on_failure → "working"` (escalate if can't fix)
3. **New `linter_worker` role** — uses `developer` tool, minimal MCP tools (StopWithError, ReportSuccess, ReportFailure, StopWithQuestion, GetCtxRec), prompt file `linter_worker.md`
4. **`LINTER_PROMPT` updated** — now check-only; removed auto-fix steps and commit instructions
5. **`LINTER_WORKER_PROMPT` added** — focused on applying formatting/linting fixes and committing them
6. **`PROMPT_FILES`** — registered `("linter_worker", LINTER_WORKER_PROMPT)`

### Flow after change
- `linting` fails → `linter_worker` (apply fixes) → `linting` (re-verify tight loop)
- `linter_worker` fails → `working` (escalate to general worker)

### Verification
`cargo build` succeeds with no errors.