## What to change

In the `PLANNER_PROMPT` constant in `zbobr/src/init.rs`, step 7 currently says "determine if the user explicitly approves this plan" and checks "the most recent (last) comment." The planner has incorrectly treated ambiguous or off-topic comments as approval (see issue reference in task description).

## Why

A user comment that doesn't directly address the plan (e.g., a question, a general acknowledgement, or a discussion about something else) was interpreted as plan approval. This causes the pipeline to proceed to implementation without actual user sign-off.

## How to apply

Revise step 7 in PLANNER_PROMPT to:
- Require an **explicit, unambiguous approval signal** — words like "approved", "yes, proceed", "looks good, go ahead", "LGTM", "confirmed" clearly directed at the plan
- Add a clear rule: if the comment could reasonably be interpreted as anything other than "yes, implement this plan", treat it as NOT approved
- Give examples of what counts as approval vs. what does not:
  - Approval: "approved", "yes proceed", "LGTM", "looks good"
  - NOT approval: questions, general feedback, topic changes, acknowledgements without explicit confirmation
- When in doubt, treat as NOT approved and call `report_intermediate` to present the plan for review

This is a prompt-only change; no code logic changes are needed.