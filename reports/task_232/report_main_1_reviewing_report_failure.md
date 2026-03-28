# Review Report: Checkbox Indentation Fix (Task #232)

## Summary
The implementation successfully fixes the checkbox indentation issue, increasing indentation from 2 to 4 spaces for top-level records. However, a **critical inconsistency** was found in the parser threshold that creates a latent bug.

## Changes Overview
- **File modified**: `zbobr-api/src/context/mod.rs` (1 file, 29 lines changed)
- **Test status**: ✅ All 42 tests pass
- **Format compliance**: ✅ cargo fmt passes
- **Test coverage**: ⚠️ No tests exercise parent-child relationships

## Issues Found

### 1. **CRITICAL: Parser Threshold Inconsistency** ❌

The parser threshold for detecting child records was changed from `>= 4` to `>= 6`, but this creates an **inconsistency with the old scaling pattern**.

**Old format (lines 2→4 spaces):**
```rust
if leading_spaces >= 4 && last_top_level_id.is_some() {  // Threshold at child level
```
- Top-level: 2 spaces
- Child: 4 spaces  
- Threshold: `>= 4` (equals child indentation)

**New format (lines 4→8 spaces):**
```rust
if leading_spaces >= 6 && last_top_level_id.is_some() {  // Threshold 2 below child level!
```
- Top-level: 4 spaces
- Child: 8 spaces
- Threshold: `>= 6` (2 spaces BELOW child indentation)

**The correct threshold should be `>= 8`** to maintain consistency with the old pattern and match actual child indentation.

**Impact**: The current `>= 6` threshold treats any line with 6-7 spaces as a child record, even though the format specifies exactly 8 spaces for children. While this doesn't break the existing tests (which only use top-level records with `parent_record_id: None`), it creates undefined behavior for any code that tries to parse 6-7 space indentation.

**Evidence from commit message**: The message states "threshold updated from >=4 to >=6 to distinguish new 4-space top-level records from 8-space children," but `>= 6` doesn't distinguish them correctly—it catches both 6-7 space malformed lines AND the proper 8-space children. A threshold of `>= 8` would be more precise.

### 2. **INCOMPLETE TEST COVERAGE** ⚠️

The test suite doesn't validate parent-child relationship parsing:
- `sample_context()` has all `parent_record_id: None`
- `md_stage_display_roundtrip()` test has no parent-child relationships
- Parser threshold change is untested in actual parent-child scenarios

This means the bug above went undetected and could silently corrupt data.

## What Works Correctly ✅

1. **Serialization (Display trait)**: Correctly outputs 4 spaces for top-level, 8 spaces for children
2. **Test assertions**: All updated to expect 4-space indentation (from 2)
3. **Documentation**: Format examples in comments correctly show 4-space indentation
4. **Code style**: Changes follow existing patterns and rustfmt compliance
5. **Scope**: All changes are focused on the indentation fix (no extraneous modifications)

## Analog Consistency

The implementation correctly follows the existing `MdStage` serialization/deserialization pattern:
- Display impl iterates records hierarchically based on `parent_record_id`
- FromStr impl reconstructs the hierarchy by detecting indentation
- The pattern is consistent, but the threshold value is wrong

## Recommended Fix

Change line 429:
```rust
// OLD (INCORRECT)
if leading_spaces >= 6 && last_top_level_id.is_some() {

// NEW (CORRECT)
if leading_spaces >= 8 && last_top_level_id.is_some() {
```

Also add a test case with parent-child records to prevent regression:

```rust
#[test]
fn md_stage_with_child_records() {
    let stage = MdStage {
        title: /* ... */,
        records: vec![
            MdRecord { id: 1, parent_record_id: None, /* ... */ },
            MdRecord { id: 2, parent_record_id: Some(1), /* ... */ },  // child of 1
        ],
    };
    let s = stage.to_string();
    // Should produce 4 spaces for id:1, 8 spaces for id:2
    let parsed: MdStage = s.parse().unwrap();
    assert_eq!(parsed.records[1].parent_record_id, Some(1));
}
```

## Summary

**Status**: Implementation is incomplete and must be fixed before merging.

The indentation values themselves are correct (4 and 8 spaces), and most of the implementation works. However, the parser threshold bug creates a latent defect that could cause data corruption when parent-child relationships are actually used. This needs to be fixed and tested before the PR can be approved.
