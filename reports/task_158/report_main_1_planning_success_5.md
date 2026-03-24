# Plan: Clean State API — Move Label Code to GitHub Backend

## Rationale

The label representation (prefix constants, label name constants, Display/From formatting as "state:done, pipeline:main") currently lives in the API layer (`zbobr-api`). This violates separation of concerns — label representation is a GitHub backend implementation detail. The plan moves all label-related code to `zbobr-task-backend-github` and replaces the string-based State API with typed methods.

## Key Design Decisions

1. **New simple serialization format** for State (used by serde and CLI FromStr): `"done"`, `"pause"`, `"ready"`, `"pending:main"`, `"running:main:bar"`. Uses colons as separators — NOT label prefixes. No `"state:"` or `"pipeline:"` prefixes. This keeps State serializable as a string (for FS backend YAML, CLI args, JsonSchema) without coupling to GitHub labels.

2. **Typed methods replace string comparisons**: All callers using `task.state == "DONE"` switch to `task.state.is_done()`. All callers using `task.state.to_string()` for display switch to `format!("{:?}", task.state)`. The `PartialEq<&str>`, `contains()`, `ends_with()` methods are removed since they only enabled brittle string comparisons.

3. **Prefix constants stay local to github backend**: `STATE_PREFIX`, `PIPELINE_PREFIX`, etc. are defined in `github.rs` only. The dispatcher defines `SIGNAL_PREFIX` locally for its signal label construction (minimal scope, avoids changing the `setup()` trait API).

4. **Analog**: The existing `signal_to_label()`/`label_to_signal()` and `flag_to_label()`/`label_to_flag()` functions in github.rs are the analog — they already handle label conversion locally in the backend. The state conversion follows the same pattern.

## Checklist Summary

5 items: move-prefix-constants → move-label-constants → rewrite-state-from-serde → update-callers → update-test-assertions
