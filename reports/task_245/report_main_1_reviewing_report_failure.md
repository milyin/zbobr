## Overall assessment
The branch is close, and the chosen analog (`instance` injection) is directionally reasonable, but the implementation does **not** fully meet the task/design requirement yet. The new code fixes the immediate `+0000` symptom in the touched paths, but it introduces a weaker timezone representation and keeps the dispatcher-to-backend propagation as a caller-side convention instead of a backend-construction guarantee.

## Analog consistency
The plan explicitly called for following the existing `instance` injection pattern. The branch mirrors that structurally (`#[config(skip_args)]`, defaulting to `None`, mutation before backend construction), but it diverges in two important ways:

1. It introduces a new raw scalar (`Option<i32>`) even though the codebase already has a domain-specific timezone type.
2. It performs the propagation in `zbobr/src/commands.rs` rather than making the backend constructor itself responsible for receiving/normalizing the dispatcher timezone.

Because of those two deviations, the new code does not feel like a clean extension of the existing config model.

## Findings

### 1. New backend configs use `Option<i32>` instead of the existing timezone type
**Files:**
- `zbobr-task-backend-github/src/config.rs:12-15`
- `zbobr-task-backend-fs/src/config.rs:9-12`
- `zbobr/src/commands.rs:203-204`
- `zbobr-task-backend-github/src/github.rs:990-993`
- `zbobr-task-backend-fs/src/fs.rs:223-236`
- existing type: `zbobr-api/src/task.rs:1-67`
- dispatcher config: `zbobr-api/src/config.rs:575-577`

The repository already defines `FixedOffsetTz`, a dedicated wrapper around `chrono::FixedOffset` with serde/clap support, and `ZbobrDispatcherConfig` already stores its timezone as `Option<FixedOffsetTz>`. The new backend fields discard that type and store only `Option<i32>` seconds.

That weakens compile-time guarantees and forces the implementation to reconstruct the timezone later with `FixedOffset::east_opt(...)`, including runtime fallback branches (`unwrap_or(parsed.timezone())` in GitHub and silent no-op fallback in FS). This is exactly the kind of inconsistent string/scalar duplication the review guidelines warn against: the same domain concept is now represented in two different ways.

**Why this matters:**
- The task asked for a timezone parameter, not a lossy numeric transport format.
- The plan also called for `Option<chrono::FixedOffset>`-style storage.
- Using the existing wrapper would eliminate both runtime recovery branches and make invalid values unrepresentable in normal construction paths.

**Suggested fix:**
Store the backend timezone as `Option<zbobr_api::task::FixedOffsetTz>` (or `Option<chrono::FixedOffset>` if the config macro supports it cleanly), inject that directly from the dispatcher config, and call `with_timezone(&tz)` without reconstructing from seconds.

### 2. Timezone propagation is still a caller-side convention, not a backend-construction guarantee
**Files:**
- `zbobr/src/commands.rs:201-205`
- `zbobr-task-backend-github/src/github.rs:1237-1241`

The task description said to set up the timezone in `TaskBackend::new()` from the dispatcher config. The branch instead mutates `tasks_config` in `zbobr/src/commands.rs` before calling `TaskBackendGithub::new(tasks_config)`. That means the backend constructor itself is still unaware of the dispatcher timezone and relies on callers to remember this extra assignment.

This is fragile for future maintenance: if another production construction path is added, it can easily forget to mirror the assignment. Centralizing the normalization at the constructor boundary is the safer pattern for an injected internal field.

**Suggested fix:**
Change the construction boundary so the backend constructor receives the dispatcher-derived timezone explicitly (or receives `ZbobrDispatcherConfig` / `FixedOffsetTz` alongside task config) and sets the internal field there. That makes timezone propagation part of backend construction rather than an external convention.

## Test coverage note
The test edits in this branch only update config construction sites so the new field compiles. They do not add any behavior-oriented coverage for timezone conversion itself. Once the type/constructor issue above is fixed, please add at least one regression test around the conversion behavior (FS backend is the easiest place to do this deterministically).

## Conclusion
I cannot approve this as-is. The immediate symptom is addressed, but the implementation misses the stronger, codebase-consistent solution: use the existing timezone domain type and make timezone propagation a constructor-level guarantee rather than a raw-seconds field populated by one caller.