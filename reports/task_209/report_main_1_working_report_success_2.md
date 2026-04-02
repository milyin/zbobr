# Fix: enforce no-spaces in Model type and validate tool-name references

## Changes

### Item 1: Enforce no-spaces in Model type (ctx_rec_27)

**zbobr-api/src/task.rs**:
- Added `Model::try_new(s: &str) -> Result<Self, String>` — rejects strings containing whitespace
- Changed `impl FromStr for Model` to delegate to `try_new`; `Err` type changed from `Infallible` to `String`
- Changed `impl Deserialize for Model` to delegate to `try_new` and propagate serde errors
- Changed `StageInfo.model` from `Option<String>` to `Option<Model>`

**zbobr-api/src/config.rs**:
- Added `Model` to imports
- Changed `ToolEntry.model` from `String` to `Model`
- Updated test fixtures: `model: "opus".parse().unwrap()`

**zbobr-api/src/context/stage_title.rs**:
- Added `Model` to imports
- Changed `MdStageTitle.model` from `Option<String>` to `Option<Model>`
- Updated `FromStr` to use `value.parse::<Model>().ok()` for model token
- Updated test fixture
- Updated comment to note the no-spaces invariant is now enforced at the type level

**zbobr-dispatcher/src/lib.rs**:
- Changed `select_provider` return type from `anyhow::Result<(ResolvedProvider, String)>` to `anyhow::Result<(ResolvedProvider, Model)>`
- Updated `tool_entry` test helper: `model: model.parse().unwrap()`
- Updated test assertions: `model.as_str() == "opus"` etc.

**zbobr-dispatcher/src/cli.rs**:
- Renamed destructured variable from `model_string` to `model`
- Updated `model_val = Some(model.clone())` (now `Option<Model>`)
- Updated `start_mcp_server` call: pass `model.clone()` directly (already `Model`)
- Updated `execute_tool` call: pass `model.as_str()`

**Other test fixtures updated**:
- `zbobr-api/src/context/mod.rs`
- `zbobr-dispatcher/src/task.rs`
- `zbobr-task-backend-github/src/separator.rs`
- `zbobr/src/init.rs`

### Item 2: Validate tool-name references eagerly (ctx_rec_28)

**zbobr-api/src/config.rs**:
- Added check in `ZbobrDispatcherConfig::validate()`: if `self.tools` is non-empty, `self.tool` must be a key in `self.tools`
- Added `ZbobrDispatcherConfig::validate_workflow_refs(&self, workflow: &WorkflowConfig) -> anyhow::Result<()>`: validates all `role.tool` and `stage.tool` references against `self.tools`

**zbobr-dispatcher/src/lib.rs**:
- `ZbobrDispatcher::validated()` now calls `self.config.validate_workflow_refs(self.workflow.config())?` after `self.config.validate()`

## Test results

All 23 test suites pass (229 tests total, 8 ignored).
