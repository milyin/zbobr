## Overall assessment

The branch largely follows the approved analog and fixes the two earlier review findings correctly: provider priority now inherits through parent chains, and unknown executor names are rejected both during config validation and in `build_executor()`. The provider/tool layering, dispatcher-side selection, and init template are all consistent with the chosen design.

I did find two remaining correctness issues that should be fixed before approval.

## Findings

### 1. Tool-name references are still only checked at runtime, not during config validation

**Where**
- `zbobr-api/src/config.rs:615-665` — `ZbobrDispatcherConfig::validate()` validates provider parents, executor names, and tool-entry provider references, but does not validate any configured tool *names* used by the dispatcher or workflow.
- `zbobr-api/src/config.rs:670-681` — `resolve_tool_name()` returns `stage.tool`, then `role.tool`, then global `dispatcher.tool` without checking that the referenced tool exists.
- `zbobr-dispatcher/src/cli.rs:346-350` — the missing validation is deferred until stage execution, where `select_provider(&tool_name)` can fail.

**Problem**
A typo in any of these places:
- `dispatcher.tool`
- `workflow.roles.<role>.tool`
- `workflow.pipelines.*.stages.*.tool`

will pass startup validation and only blow up when that stage is actually run. That is a regression in configuration robustness: the new tool indirection moved more behavior into named references, but validation only covers the tool definitions themselves, not the references to them.

**Why it matters**
This feature replaced the old direct `tool/model/plan_mode` configuration with a named-tool indirection specifically to make configuration safer and more flexible. Leaving those references unchecked means simple misconfiguration survives build/startup and turns into a runtime failure in the middle of task processing.

**Suggested fix**
Extend validation so every resolved tool reference is checked eagerly:
1. `dispatcher.tool` must exist in `self.tools`
2. every `RoleDefinition.tool` must exist
3. every `StageDefinition.tool` must exist
4. optionally reject empty tool entry lists as invalid too, since they currently fail later with the misleading "all providers excluded" error path

This is also a good place to strengthen type specificity: the code already has a dedicated concept of a named tool reference, so config validation should make those strings safe before execution starts.

### 2. Persisted stage-title parsing still assumes tool/model values never contain spaces, which conflicts with arbitrary model strings

**Where**
- `zbobr-api/src/context/stage_title.rs:151-176`
- especially `zbobr-api/src/context/stage_title.rs:158-159`, which explicitly states: `tools and models never have spaces`

**Problem**
The task explicitly changed `model` from a closed enum to an arbitrary string. But the markdown stage-title parser still distinguishes `tool`/`model` from the timestamp by assuming any backtick token containing a space must be the timestamp.

That means stage metadata is no longer robust for the new model semantics:
- a model containing spaces cannot round-trip through `MdStageTitle`
- similarly, any future provider/tool name with spaces would also be ambiguous
- the persisted markdown representation is now stricter than the config/runtime model type, so valid runtime values can become unparsable once stored

**Why it matters**
`StageInfo` is persisted and parsed back through the stage-title machinery. This change made model names open-ended, so serialization/parsing must no longer rely on the old enum-era invariant that models never contain spaces.

**Suggested fix**
Make the stage-title format unambiguous for arbitrary strings instead of depending on the no-spaces assumption. For example:
1. parse the timestamp from the final backtick token position rather than by content heuristics, or
2. give tool/model/timestamp labeled fields in the markdown format, or
3. explicitly validate and reject unsupported characters in tool/model strings before persistence if the format intentionally remains restricted

Right now the implementation silently keeps the old format assumptions while widening the accepted model domain, which is inconsistent.

## Analog consistency

The overall structure remains aligned with the approved analog:
- provider inheritance is resolved centrally in config
- dispatcher selection handles priority/round-robin/exclusion
- stage/role now resolve a single named tool
- executor wiring receives raw model strings

The main inconsistencies are both around validation/persistence boundaries rather than the high-level architecture:
- named tool references are not validated as eagerly as the rest of the config graph
- the stage-title persistence layer still behaves like models come from a closed, no-spaces enum even though the model type was intentionally opened up

## Checklist status

All checklist items appear implemented, and there were no remaining unchecked items to mark. The failure report is due to the correctness issues above, not missing checklist work.

## Conclusion

The branch is close, but I do not recommend approval yet. The missing tool-reference validation leaves misconfiguration to fail during live stage execution, and the stage-title parser still encodes the old "models never have spaces" assumption despite models now being arbitrary strings.