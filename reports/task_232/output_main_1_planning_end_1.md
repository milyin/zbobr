The plan has been submitted. Here's a summary:

**Bug:** `MdStage::fmt()` in `zbobr-api/src/context/mod.rs` renders records with only 2 spaces of indentation. GitHub Markdown requires 4 spaces for items to nest as sub-items under the preceding bullet.

**Fix (single file):**
1. **Renderer** – change top-level record indent 2→4 spaces, child indent 4→8 spaces
2. **Parser** – raise child-detection threshold from `>= 4` to `>= 5` so 4-space top-level records aren't misclassified as children; old 2-space records in existing issues still parse correctly (backward compat)
3. **Tests** – update `serialize_basic` assertions and hardcoded test-input strings to match the new 4-space format