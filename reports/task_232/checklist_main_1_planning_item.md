## What and Why

The bug is in `MdStage::fmt()` (Display impl) in `zbobr-api/src/context/mod.rs`.

Currently, top-level records are rendered with 2 spaces of indentation (`"  {}"`) and child records with 4 spaces (`"    {}"`). GitHub Flavored Markdown requires 4 spaces for a list item to be treated as a sub-item of the preceding bullet. Since stage titles are rendered as `- YYYY-MM-DD ...` (top-level bullets), their record sub-items need 4 spaces to visually nest under them.

### Changes required

**In `MdStage::fmt()` (Display):**
- Change top-level record indent: 2 spaces → 4 spaces
- Change child record indent: 4 spaces → 8 spaces

**In `MdStage::from_str()` (parser):**
- The child-detection threshold (`leading_spaces >= 4`) must be increased so that 4-space (top-level) records are not mistakenly treated as children. Change to `>= 5` (anything beyond 4 spaces = child). This also maintains backward compatibility: old 2-space-indented records from existing GitHub issues will still be parsed as top-level (2 < 5).

**Test updates in `zbobr-api/src/context/mod.rs`:**
- `serialize_basic` test (lines 812–822): update all assertions from `"  - "` to `"    - "` to match the new renderer output.
- The hardcoded test-input strings in `parse_ignores_blockquote_comments` (lines 944, 949) and `parse_error_on_missing_id` (line 979) currently use 2-space indentation. Update these to 4 spaces to keep test data consistent with the canonical new format (the parser still accepts both, so tests remain correct either way, but consistency matters for readability).

The `parse_error_on_record_before_stage` test (line 964) does not depend on indentation level—the error is triggered by the absence of a stage header—so it does not need updating.
