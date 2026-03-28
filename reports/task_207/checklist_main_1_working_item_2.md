In zbobr-api/src/context/mod.rs `from_stage_context`, two issues:
1. `output_link` is not transformed via `report_url` (only `prompt_link` is)
2. `output_link` is not cleared when `for_prompt=true` (only `prompt_link` is cleared)
Fix: apply `report_url` to `output_link` just like `prompt_link`, and clear `output_link` when `for_prompt=true`.