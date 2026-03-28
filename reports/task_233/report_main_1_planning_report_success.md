# Plan Summary

## Approach
Both changes are prompt-only modifications to constants in `zbobr/src/init.rs`.

## Change 1: TESTER_PROMPT — allow formatting fixes
Remove the blanket "Do not modify files" restriction for the tester. Allow the tester to run the project's formatter (e.g., `cargo fmt`) when formatting-only issues are detected, commit the fix, and continue to report success. Logic/code changes remain prohibited — only deterministic formatter output is allowed.

## Change 2: PLANNER_PROMPT — stricter approval check
Revise step 7 to require explicit, unambiguous approval signals ("approved", "yes proceed", "LGTM", etc.). Add clear examples of what counts as approval vs. what does not. When in doubt, treat the comment as NOT approved and call `report_intermediate` instead.

## Key constraints
- Prompt-only changes, no Rust code logic affected
- CLAUDE.md rule: do not mention pipeline workflow details (stage names, transitions) in prompts