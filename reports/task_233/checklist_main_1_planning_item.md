## What to change

In the `TESTER_PROMPT` constant in `zbobr/src/init.rs`, the tester is currently prohibited from modifying files ("Do not modify files" rule in the Important Notes section). This causes the tester to reject jobs for pure formatting issues, triggering a full worker/reviewer loop unnecessarily.

## Why

Formatting fixes (running `cargo fmt`, `prettier`, `black`, etc.) are mechanical and low-risk. Requiring a full loop just to fix whitespace/style is wasteful. The tester is the best place to detect and fix them since it already runs formatters as part of CI validation.

## How to apply

Modify the "Do not modify files" note in TESTER_PROMPT to:
- Allow the tester to apply **formatting-only** fixes (running the project's formatter tool, e.g. `cargo fmt`)
- Require the tester to commit the formatting fix with a clear message (e.g., `chore: fix formatting`)
- After committing, the tester should continue and report success (not failure) for formatting-only issues that were fixed
- Keep the prohibition on logic/code changes — only mechanical formatter output is allowed

The distinction is: if a formatter tool can automatically produce the fix deterministically, the tester can apply it; if it requires reasoning about code, it should not.