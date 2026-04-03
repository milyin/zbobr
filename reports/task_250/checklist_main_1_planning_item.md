In `zbobr/src/init.rs`, function `default_config_toml()` (around lines 138-199), add a new "drudge" tool entry to the tools IndexMap. Follow the "helper" tool pattern.

**What:** Define a new tool called "drudge" with two providers:
- copilot provider with model `gpt-5-mini`, default priority (None)
- claude provider with model `claude-haiku-4.5`, priority 0 (backup)

**Why:** The linting stage needs the cheapest available models since formatting/linting checks are mechanical tasks that don't require advanced reasoning.

**Pattern to follow:** The "helper" tool definition nearby — same ToolEntry struct with provider, model, and priority fields. Place "drudge" after "helper" in the IndexMap.