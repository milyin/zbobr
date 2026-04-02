# Plan Finalized

User approved the plan from the previous session (ctx_rec_1). 

## Approach chosen

Layered two-level abstraction: providers (named executor configs with inheritance) and tools (named lists of provider+model pairs). Provider selection uses priority + round-robin within a tier, with temporary exclusion on failure.

## Key design decisions

- `providers` and `tools` live under `[dispatcher]` in zbobr.toml to keep all dispatch config co-located
- `Model` and `Tool` become open string newtypes — no more closed enum, no more `model_name_for_tool()` mapping table
- `plan_mode` moves entirely to provider level; stages/roles no longer control it
- Exclusion state is runtime-only (Mutex<HashMap> on dispatcher), not serialized
- Provider inheritance resolved at config-build time (not dispatch time)
- Retry-with-exclusion logic lives in cli.rs stage runner

## Checklist items created (6)

1. Config types in zbobr-api/src/config.rs (ProviderDefinition, ToolEntry, updated structs, resolve_tool_name)
2. Task types in zbobr-api/src/task.rs (Tool/Model as string newtypes, remove model_name_for_tool)
3. Executor configs + ToolExecutor trait (remove default_model, add access_key, model as &str)
4. Dispatcher provider selection logic in zbobr-dispatcher/src/lib.rs
5. Stage runner in zbobr-dispatcher/src/cli.rs (replace 3-call resolution, retry-with-exclusion)
6. Init template in zbobr/src/init.rs (providers/tools config, updated roles)
