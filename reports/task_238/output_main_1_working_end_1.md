All three checklist items completed. Here's a summary of what was implemented:

## Secret struct refactor — done

**Core change**: `Secret` moved from `zbobr-api` (enum) to `zbobr-utility` (struct with private fields).

**New API**:
- `Secret::value(s)` / `Secret::env(var)` — constructors (replaces `Secret::Value(s)` / `Secret::Env(s)`)
- `resolve(&mut self) -> Result<&str>` — fetches and caches the value; must be called before `as_ref()`
- `AsRef<str>` — panics if `resolve()` was never called (early-panic design)
- `is_resolved() -> bool` — for completeness
- `Clone` preserves resolved state

**Migration pattern**:
- All `validate()` methods now take `&mut self` and call `resolve()` on their token fields
- Runtime code uses `as_ref()` after validated/pre-resolved state
- `ZbobrDispatcher::validated()` also resolves the copilot token
- Integration tests pre-resolve the dispatcher's `agent_github_token` before wrapping in `Arc` (since they bypass `validated()` due to McpTester tool not passing the model compatibility check)