# Fix: Checkbox Indentation (issue #232)

## Problem
Checkbox items in the MdStage renderer were indented with only 2 spaces, making them appear at the same nesting level as the stage header `- ` in GitHub Markdown. They should be proper sub-items (4 spaces) to be rendered nested under the stage.

## Changes (zbobr-api/src/context/mod.rs)

### Renderer (`fmt::Display` for `MdStage`)
- Top-level records: `"  {}"` → `"    {}"` (2 → 4 spaces)
- Child records: `"    {}"` → `"        {}"` (4 → 8 spaces)

### Parser (`FromStr` for `MdStage`)
- Threshold updated: `leading_spaces >= 4` → `leading_spaces >= 6`
- This correctly distinguishes: 4-space top-level (< 6) from 8-space children (>= 6)
- Backward compatible: old 2-space records still parsed as top-level (2 < 6)

### Tests
- Updated `serialize_basic` assertions from 2-space to 4-space prefixes
- Other parse tests using hardcoded 2-space text still pass (outer context parser uses `trimmed` so indentation is irrelevant there)

## Verification
- All 31 `zbobr-api` context tests pass
- Full workspace build succeeds
