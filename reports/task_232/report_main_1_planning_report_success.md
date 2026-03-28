## Root Cause

In `zbobr-api/src/context/mod.rs`, `MdStage::fmt()` renders top-level stage records with only 2 spaces of indentation. GitHub Flavored Markdown requires 4 spaces for a list item to nest as a sub-item under the preceding bullet (`- stage-title`). This causes checkboxes to appear at the same nesting level as the stage header rather than indented beneath it.

## Approach

Single-file fix in `zbobr-api/src/context/mod.rs`:

1. **Renderer (`MdStage::fmt`)**: change top-level record indent 2→4 spaces and child record indent 4→8 spaces.
2. **Parser (`MdStage::from_str`)**: raise child-detection threshold from `>= 4` to `>= 5`, so 4-space (new top-level) records aren't misclassified as children. Old 2-space-indented records in existing GitHub issues continue to parse correctly (2 < 5 → treated as top-level).
3. **Tests**: update `serialize_basic` assertions and hardcoded input strings in parsing tests to reflect the new 4-space format.

No other files are affected.
