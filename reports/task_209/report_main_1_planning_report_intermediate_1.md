## Two fixes required by the review

### Fix 1: Enforce no-spaces in `Model` + use `Model` consistently

**Affected files**: `zbobr-api/src/task.rs`, `zbobr-api/src/config.rs`, `zbobr-api/src/context/stage_title.rs`, `zbobr-dispatcher/src/lib.rs`, `zbobr-dispatcher/src/cli.rs`

The user confirmed: model names are not allowed to contain spaces. The stage-title parser's assumption is correct — it just needs to be enforced at the type level rather than left implicit.

Changes:
- Add `Model::try_new(s: &str) -> Result<Self, String>` that rejects strings containing whitespace.
- Change `Model::from_str` to use `try_new` (error type: `String` instead of `Infallible`).
- Change `Model::deserialize` to use `try_new` and return a serde error on invalid input.
- Change `ToolEntry.model: String` → `ToolEntry.model: Model` in `config.rs`.
- Change `StageInfo.model: Option<String>` → `StageInfo.model: Option<Model>` in `task.rs`.
- Update the comment in `stage_title.rs:159` to say the invariant is enforced at type level.
- Change `select_provider` return type: `(ResolvedProvider, String)` → `(ResolvedProvider, Model)`.
- Fix all downstream call sites (test helpers, CLI, anything constructing `ToolEntry`/`StageInfo` with raw string models).

### Fix 2: Validate tool-name references at config load time

**Affected files**: `zbobr-api/src/config.rs`, `zbobr-dispatcher/src/lib.rs`, `zbobr-dispatcher/src/workflow.rs`

Currently `validate()` checks providers and tool-entry provider references but does not check whether tool *names* referenced by `dispatcher.tool`, roles, or stages actually exist in `self.tools`.

Changes:
- In `ZbobrDispatcherConfig::validate()`, add check: `self.tool` must exist in `self.tools`.
- Add `ZbobrDispatcherConfig::validate_workflow_refs(&self, workflow: &WorkflowConfig) -> anyhow::Result<()>` that iterates all roles and all pipeline stages and rejects any `Some(tool)` that is not a key in `self.tools`.
- Add a `config()` getter on `Workflow` to expose `&WorkflowConfig`.
- In `ZbobrDispatcher::validated()`, chain `self.config.validate_workflow_refs(self.workflow.config())?` after `self.config.validate()`.

### Verification
- `cargo test -p zbobr-api` and `cargo test -p zbobr-dispatcher` pass.
- All 20 existing tests continue to pass.
- New tests: Model rejects spaces, validate() rejects unknown global tool, validate_workflow_refs() rejects unknown tool in role/stage.
