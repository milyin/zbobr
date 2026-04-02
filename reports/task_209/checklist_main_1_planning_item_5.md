# Init template update (zbobr/src/init.rs)

## What to change

In `default_config_toml()`, update the dispatcher section to include:

**providers** (under dispatcher):
- `claude`: executor = "claude", priority = 10
- `copilot`: executor = "copilot", priority = 10
- `claude_planner`: parent = "claude", plan_mode = true
- `copilot_planner`: parent = "copilot", plan_mode = true

**tools** (under dispatcher):
- `smart`: [{provider="copilot", model="claude-opus-4.6"}, {provider="claude", model="claude-opus-4-6"}]
- `planner`: [{provider="copilot_planner", model="claude-opus-4.6"}, {provider="claude_planner", model="claude-opus-4-6"}]
- `silly`: [{provider="copilot", model="gpt-5-mini"}, {provider="claude", model="claude-haiku-4-5"}]

Set global dispatcher `tool = "smart"` (default for all roles unless overridden).

In `default_workflow()`, update roles:
- `planner` role: set `tool = "planner"`, remove `default_plan_mode = Some(true)`
- All other roles: no `tool` field (inherit global "smart")
- Remove all `default_tool`, `default_model`, `default_plan_mode` fields from all roles

Remove `default_model` from executor configs in the toml (since executors no longer have that field).

## Why

The init template is what new users see first. It needs to demonstrate the full providers/tools pattern including multi-provider selection, priority, and the planner pattern (where plan_mode comes from the provider, not the role). Each tool shows selection between copilot and claude to demonstrate the fallback logic.
