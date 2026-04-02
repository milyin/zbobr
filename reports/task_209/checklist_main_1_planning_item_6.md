## What to change

In `zbobr-api/src/task.rs`:
- Add `Model::try_new(s: &str) -> Result<Self, String>` that returns `Err` if the string contains any whitespace.
- Change `impl FromStr for Model` to delegate to `try_new`; the `Err` type becomes `String` (not `Infallible`).
- Change `impl Deserialize for Model` to delegate to `try_new` and propagate a serde error on invalid input.
- Change `StageInfo.model` from `Option<String>` to `Option<Model>`.

In `zbobr-api/src/config.rs`:
- Change `ToolEntry.model` from `String` to `Model`.
- Fix any places constructing `ToolEntry` with a raw string literal for `model` (e.g. in tests or init template) — use `Model::try_new(...)` or `"string".parse::<Model>()`.

In `zbobr-dispatcher/src/lib.rs`:
- Change `select_provider` return type from `anyhow::Result<(ResolvedProvider, String)>` to `anyhow::Result<(ResolvedProvider, Model)>` — the model is now returned as typed `Model`, not a raw string.
- Update the final `Ok((rp, entry.model.clone()))` accordingly.

In `zbobr-dispatcher/src/cli.rs`:
- Update the destructuring of `select_provider` result — the model value is now a `Model`, adapt downstream usage.

In `zbobr-api/src/context/stage_title.rs`:
- Update the comment on line ~159 from "tools and models never have spaces" to note the invariant is now enforced at the type level by `Model`.

## Why
The stage-title persistence layer distinguishes backtick tokens by presence/absence of spaces (space → timestamp, no space → tool or model). This assumption is correct but was previously implicit. Making `Model` reject spaces at construction time makes the constraint explicit and catches misconfiguration at config parse time rather than at persistence time.