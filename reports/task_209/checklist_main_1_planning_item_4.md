# Stage runner refactor (zbobr-dispatcher/src/cli.rs)

## What to change

In the stage execution path (around lines 349-357 and 1009-1012 where the three-call resolution currently happens):

Replace:
```
let tool = config.tool_for_stage(stage_def, workflow);
let model = config.model_for_stage(stage_def, workflow);
let plan_mode = config.plan_mode_for_stage(stage_def, workflow);
```

With:
```
let tool_name = config.resolve_tool_name(stage_def, workflow);
let (resolved_provider, model) = dispatcher.select_provider(&tool_name)?;
let plan_mode = resolved_provider.plan_mode;
```

On executor failure that indicates a recoverable provider issue (connectivity error, rate limit, quota exhaustion):
- Call `dispatcher.exclude_provider(&resolved_provider.name)`
- Retry with a new provider selection if available; otherwise propagate the error

Update `WorkerRequest` struct (around line 1556):
- Replace `tool: Tool`, `model: Model`, `plan_mode: bool` fields with `provider_name: String`, `model: String`, `plan_mode: bool`

Update `StageInfo` population:
- Set `tool` to `Some(provider_name)` (for display/logging)
- Set `model` to `Some(model_string)`

## Why

The stage runner is the integration point between config resolution and executor invocation. The retry-with-exclusion logic here is what makes the fallback system work: when a provider fails, it gets excluded and the next call to select_provider will skip it and pick the next available one.

## Note on error classification

For now, treat all executor errors as potentially recoverable (exclude the provider). We can refine this later to distinguish permanent config errors from transient connectivity failures.
