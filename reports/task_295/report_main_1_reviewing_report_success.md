Review result: approved.

Overall assessment:
- The final implementation satisfies the task requirement: logs are no longer initialized before CLI parsing, so log output is suppressed by default unless `--logs` is passed.
- The follow-up fix correctly integrates `--logs` with the existing global CLI hoisting mechanism in `zbobr-dispatcher::parse_cli`, so the flag now follows the repository’s established pattern for root-level options.
- The change set is focused and limited to the two files directly involved in CLI definition and global argument hoisting.

Analog / pattern consistency:
- The appropriate analog in this codebase is the existing global-option pattern implemented via `zbobr-dispatcher/src/cli.rs::GlobalArgs` and `parse_cli`.
- The initial one-file plan understated that analog, but the final code follows it correctly.
- Duplicating the flag in both `zbobr/src/main.rs::Cli` and `zbobr-dispatcher/src/cli.rs::GlobalArgs` is consistent with the existing `config_file` pattern: `GlobalArgs` is used for hoisting, while the root `Cli` owns the actual parsed value for the binary.

Code quality / correctness review:
- `zbobr/src/main.rs`
  - `Cli` now declares `logs: bool` as a root-level flag.
  - Tracing initialization now happens after CLI parsing.
  - The filter is `off` by default and falls back to `RUST_LOG` / `info` only when `--logs` is enabled.
  - This directly implements the requested behavior.
- `zbobr-dispatcher/src/cli.rs`
  - `GlobalArgs` now includes `logs: bool`, which makes the flag part of the hoisting set discovered by `parse_cli`.
  - This resolves the prior correctness issue where `zbobr task process --logs` would fail even though `zbobr --logs task process` worked.

Compile-time / robustness review:
- No new weakly-typed configuration surfaces were introduced beyond the normal clap boolean flag pattern already used in the codebase.
- The implementation is robust against partial CLI changes because the hoisting mechanism derives the hoisted flag set from `GlobalArgs`; adding `logs` there keeps the parser behavior aligned with the declared global arguments.
- I did not find any stronger existing domain type that should have been used for this flag.

Extraneous changes:
- None found. Both modified files are directly related to the task.

Checklist review:
- There were no remaining unchecked checklist items in the provided task context.
- The completed checklist items are consistent with the final implementation state.

Findings:
- No blocking issues found.