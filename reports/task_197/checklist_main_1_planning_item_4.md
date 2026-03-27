# Verification Steps

## 1. Compile check
Run from the repo root:
```
cargo check
```
Ensure no errors. The only risky part is the new `ReportIntermediate` enum variant in the planner MCP list — it's already imported in the same file (used by worker/reviewer), so this should compile cleanly.

## 2. Inspect generated prompt
Run `zbobr init` in a temp directory (or check the test that covers prompt generation) to confirm `prompts/planner.md` includes:
- `report_intermediate` in the Access Model section
- Architecture-level language in steps 3–4 (no "explore all code details")
- Two-step finalization in steps 8–9 (intermediate first, success after confirmation)

## 3. Optional: check tool list
In a planner session, the `report_intermediate` tool should appear in the available MCP tool list alongside `report_success`.
