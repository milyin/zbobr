# Add parent_record_id Field to ContextRecord

## What to change
Modify the `ContextRecord` struct in `zbobr-api/src/task.rs` to add an optional `parent_record_id` field.

## Current structure
The `ContextRecord` struct (around line 133-143) currently has:
- id: u64
- record_type: ContextRecordType
- brief: String
- report_link: Option<String>

## New field to add
```
parent_record_id: Option<u64>
```

This should be:
- An optional field (wrap in `Option<u64>`)
- Serde-compatible with defaults (use `#[serde(default, skip_serializing_if = "Option::is_none")]`)
- Documented to explain it references the id of a parent report record

## Why
This field establishes the parent-child relationship between checklist items and the reports they elaborate on. It allows the context structure to represent the hierarchy explicitly.

## How to apply
- Locate the `ContextRecord` struct definition
- Add the `parent_record_id: Option<u64>` field with appropriate serde attributes
- Add documentation comment explaining the field's purpose
- This change is backward-compatible because it's optional and marked with `skip_serializing_if`