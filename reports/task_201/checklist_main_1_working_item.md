Fix the 4 issues identified by the reviewer:

1. Update trait doc in zbobr-api/src/backend.rs line 160: change "signals/tools/models as Labels" to reflect that signal/pipeline/stage are now in params, only state/flags remain as labels.

2. Fix stale comment in apply_state_change (github.rs line 353): "remove all existing state:/pipeline:/stage: labels" → "remove all existing state: labels"

3. Add KEY_PIPELINE, KEY_STAGE, KEY_SIGNAL constants at top of github.rs and replace all "pipeline", "stage", "signal" string literals in params operations.

4. Add .map(str::trim).filter(|s| !s.is_empty()) when extracting pipeline_param and stage_param from params_map to reject empty/whitespace values.
