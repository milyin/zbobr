Overall assessment:

The code changes for the single-repository simplification are largely consistent with the planned analogs and with existing backend/dispatcher patterns. In particular:
- `TaskIdentity` is correctly simplified to task/work-branch identity only.
- Repo selection/base branch now come from the repo backend, which matches the requested architecture.
- Preparator removal is implemented consistently across workflow/config/public interfaces.
- The GitHub repository normalization/parser is reasonable for now; I am not treating stricter parsing as a blocker, per the explicit user instruction.

However, there is still one must-fix issue before accepting this task.

Must-fix issue:

1. README token/config documentation still contradicts the current implementation

The branch updated `README.md`, but several surviving sections still describe an old token model and obsolete validation behavior that no longer matches the code.

Examples:
- `README.md:293-321` still uses the old "owner token" terminology and claims validation of `ZBOBR_AGENT_GH_TOKEN` vs `ZBOBR_OWNER_GH_TOKEN`.
- `README.md:309-311` says startup validation requires `ZBOBR_AGENT_GH_TOKEN` to be set and different from `ZBOBR_OWNER_GH_TOKEN`.
- In reality, `zbobr-api/src/config.rs:573-582` only resolves `agent_github_token`; it does not enforce those env vars or a "must differ" rule.
- `README.md:318-321` points readers at `zbobr-dispatcher/src/backend/github.rs`, which does not exist in this codebase and no longer reflects the repo-backend split.
- `README.md:331-337` still documents `ZBOBR_AGENT_GH_TOKEN` / `ZBOBR_COPILOT_GITHUB_TOKEN`-style env var behavior as if it were the canonical current interface, which is not how the current config docs elsewhere in the same README are framed.

Why this matters:
- This task explicitly includes updating documentation/examples for the simplified single-repo design.
- These remaining README sections are user-facing and materially misleading: they describe validation guarantees that do not exist and reference outdated architecture/file paths.
- The result is an inconsistent public contract: earlier README sections describe the new `[tasks]`/`[repo]` model correctly, while later sections contradict it.

Suggested fix:
- Do one final cleanup pass on the README token/security section so it matches the current implementation exactly.
- Remove or rewrite the obsolete "owner token" / `ZBOBR_OWNER_GH_TOKEN` wording.
- Remove claims about validation that are not actually enforced.
- Replace stale file references with current ones, or remove code-path references if they are not stable enough for README-level docs.
- Keep the terminology aligned with the current backend split: task token, repo token, agent token, copilot token.

Analog consistency assessment:

The implementation itself follows the right analog and now looks coherent: backends own repository routing, task objects no longer carry per-task destination routing, and dispatcher prompt injection uses repo backend state. I did not find a code-level regression in the single-repo simplification itself beyond the documentation mismatch above.

Follow-up task (non-blocking, per user instruction): GitHub repo URL parsing hardening

The current `parse_github_repo()` / normalization logic is reasonable enough to accept for this task, but a future dedicated task should replace the ad-hoc string parsing with a more explicit repository-reference type and clearer parsing rules.

Recommended scope for that follow-up:
- Introduce a dedicated parsed repository type (e.g. validated `owner/repo`) instead of passing raw `String` after normalization.
- Define supported input forms explicitly (plain `owner/repo`, canonical HTTPS URL, canonical SSH URL).
- Centralize parsing/normalization in one reusable type/API rather than using raw strings at boundaries.
- Decide whether prompt-preview / dry-run paths should fail hard on invalid repo config instead of falling back.

Conclusion:

Failing review for one must-fix documentation issue only. Once the README token/config section is made consistent with the current implementation, I would consider this task acceptable.