# Test Plan: Latest Fix Commit (3a209654)

## Analysis

The latest commit (`3a209654 fix: enforce no-spaces in Model type and validate tool-name references eagerly`) introduced three new pieces of logic with **zero test coverage**:

1. **`Model::try_new()`** — whitespace rejection in model names, plus delegated `FromStr` and `Deserialize`
2. **`validate()` global tool check** — verifies `dispatcher.tool` exists in `self.tools` when tools are configured
3. **`validate_workflow_refs()`** — validates that role and stage tool-name references exist in `self.tools`

The prior test rounds (20 + 3 = 23 tests) covered the initial provider/tool/selection logic and the priority-inheritance/executor-validation fixes, but none of the code from this latest commit is tested.

## Test Groups

### Group 1: `Model::try_new()` — 5 tests (zbobr-api/src/task.rs)
- Valid model name accepted
- Space in model name rejected
- Tab in model name rejected
- `FromStr` delegates rejection correctly
- `Deserialize` delegates rejection correctly

### Group 2: `validate()` global tool check — 2 new tests (zbobr-api/src/config.rs)
- Unknown global tool rejected when tools map is non-empty
- Empty tools map allows any global tool value (backward compat)

### Group 3: `validate_workflow_refs()` — 4 tests (zbobr-api/src/config.rs)
- Unknown role tool reference rejected
- Unknown stage tool reference rejected
- Valid references pass
- `None` tool references pass (no validation needed)

## Total: 11 new tests across 2 files
