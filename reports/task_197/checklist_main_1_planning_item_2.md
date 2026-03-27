# Update PLANNER_PROMPT: Workflow Steps 3–4

## File
`zbobr/src/init.rs`, constant `PLANNER_PROMPT` (lines 433–434)

## Current text (steps 3 and 4)
```
3. **Search for analogous functionality in the codebase BEFORE designing the plan.** Look for existing code that does something similar to what the task requires — similar features, modules, patterns, or workflows. This is critical: the implementation must follow the same approaches, conventions, and style as the existing analogous code. Identify the analog explicitly in your plan so the worker and reviewer can reference it.
4. Your current working directory is already the repository with the work branch checked out. Explore the codebase and design a step-by-step implementation plan that follows the patterns and style of the identified analog if found.
```

## Replacement text
```
3. **Identify the closest analog in the codebase BEFORE designing the plan.** Find the existing module, struct, or pattern most similar to what the task requires. Name the analog (file and module/type) explicitly — do not explore implementation details beyond what is needed to confirm the analogy. This is critical: the implementation must follow the same approaches, conventions, and style as the analog.
4. **Design an architecture-level plan.** Describe which components or modules need to be added or changed, what interfaces or data flows are affected, and which patterns from the analog to follow. Focus on *what* changes and *why* — avoid code snippets and low-level file details. The worker will look up the details; the plan should give clear direction without prescribing exact implementation.
```
