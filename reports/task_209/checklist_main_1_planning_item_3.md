# Dispatcher: provider selection logic (zbobr-dispatcher/src/lib.rs)

## What to add

Add runtime state fields to `ZbobrDispatcher`:
- `excluded_providers: Arc<Mutex<HashMap<String, Instant>>>` — maps provider name to when exclusion expires
- `round_robin_state: Arc<Mutex<HashMap<String, usize>>>` — per-tool counter for round-robin within a priority tier

Add method `select_provider(&self, tool_name: &str) -> anyhow::Result<(ResolvedProvider, String)>`:
1. Look up `tool_name` in `config.tools` → get Vec<ToolEntry>
2. Filter out entries whose provider is currently excluded (check excluded_providers map, remove expired entries)
3. Group remaining entries by provider priority (from resolved providers map)
4. Select the highest-priority group
5. Within that group, use round_robin_state counter to pick the next entry (increment counter mod group size)
6. Return `(resolved_provider, model_string)`
7. If no providers available (all excluded), return error

Add method `exclude_provider(&self, provider_name: &str)`:
- Insert provider_name → `Instant::now() + Duration::from_secs(config.provider_exclusion_secs)` into excluded_providers map

Update `build_executor()`:
- Accept a `ResolvedProvider` instead of looking up tool/model from config
- Extract executor type string from provider, match to `ClaudeExecutor`/`CopilotExecutor`/`McpTesterExecutor`
- Pass access_key override to claude executor if present in provider

## Why

The selection logic must be centralized in the dispatcher (not in cli.rs) so it can maintain round-robin state and exclusion state across multiple stage runs within a session. The Arc<Mutex<>> pattern matches existing patterns in the codebase for shared mutable state.
