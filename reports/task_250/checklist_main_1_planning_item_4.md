In `zbobr/src/init.rs`, modify the TESTER_PROMPT constant to remove all formatting/linting responsibilities.

**What to remove from TESTER_PROMPT:**
- Step 2 bullet: "Identify code formatting and linting requirements"
- Step 3 bullet: "Run formatting/linting checks to ensure code quality"
- Step 4 entirely (the step about fixing formatting/linting issues)
- Step 6 bullet: "Formatting/linting issues (and whether you fixed them)"
- Important Notes bullet: "Formatting fixes are allowed..."
- Important Notes bullet: "Do not modify logic" (this was specific to the formatting context)

**What to add:**
- A note in the workflow or important notes section: "Linting and formatting checks are handled by a separate stage — do not run them here."

**Why:** Since linting is now a separate stage that runs before testing, the tester should focus exclusively on functional testing. This avoids duplicate work and clarifies responsibilities.

**Renumber steps** after removing step 4 (steps 5-7 become 4-6).