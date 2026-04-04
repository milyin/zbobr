Plan approved by user. Created 6 implementation checklist items to fix incorrect timezone on interspersed comments.

**Approach**: Follow the existing `instance` injection pattern — add `timezone: Option<chrono::FixedOffset>` to both backend configs, inject from dispatcher config at construction, and apply `.with_timezone()` when building Comment objects.

**Key design decision**: Reuse the same skip-args + injection-at-construction pattern already established for the `instance` field, keeping the codebase consistent.

**Analog**: The `instance` field in both backend configs — same annotation, same injection site in `commands.rs`.