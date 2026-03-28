In `zbobr-dispatcher/src/prompts.rs`, the constants `VAR_DESTINATION_REPOSITORY`, `VAR_DESTINATION_BRANCH`, and `VAR_WORK_BRANCH` have the same string values as the new `PARAM_*` constants in `zbobr-api`.

These are used as prompt template placeholder names, not as task parameter keys. Since the string values are identical, they can either:
1. Be redefined as aliases: `pub const VAR_DESTINATION_REPOSITORY: &str = zbobr_api::PARAM_DESTINATION_REPOSITORY;`
2. Or remain as independent constants with the same values

The preferred approach is option 1 (aliasing from the shared constant) to make the relationship explicit and avoid future drift. This also removes duplicate string literals.

Why: Keeps the dispatcher's prompt variable names in sync with the API parameter names they represent, without duplicating the string definitions.