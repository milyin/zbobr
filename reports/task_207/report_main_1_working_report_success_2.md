# Fix: output_link URL mapping and for_prompt omission

## Issues Fixed

The reviewer identified two problems in `zbobr-api/src/context/mod.rs` `from_stage_context`:

### 1. output_link not URL-mapped
Previously only `prompt_link` was transformed via the `report_url` function to convert filenames to full GitHub URLs. `output_link` was stored as a raw filename (`output_main_1_working_end.md`) and would appear broken in GitHub markdown.

**Fix**: Applied the same `report_url` transformation to `output_link` by iterating both links together.

### 2. output_link appearing in prompt context
Previously when `for_prompt=true`, only `prompt_link` was cleared. `output_link` remained in the serialized context and appeared in agent prompts unnecessarily.

**Fix**: Also clear `output_link` when `for_prompt=true`.

## Tests Added
- `for_prompt_also_omits_output_link`: verifies output_link is None after for_prompt serialization
- `output_link_url_mapped_via_report_url`: verifies output_link filename is expanded to full URL via report_url

All 44 tests pass.