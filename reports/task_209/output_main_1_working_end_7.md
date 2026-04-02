All done. The integration test helpers now:
1. Define a `"mcp-tester"` provider (`executor = "mcp-tester"`) and a `"mcp-tester"` tool entry in the dispatcher config
2. Call `.validated()` on every dispatcher build, matching production startup behavior
3. Stage definitions referencing `tool: "mcp-tester"` now resolve correctly through `select_provider()`