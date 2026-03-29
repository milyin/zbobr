All work is complete. Here's what was accomplished in this session:

**ctx_rec_6** — Added `instance: String` to `StageInfo` and `MdStageTitle`, updated the stage title format from `pipeline:run_id:**stage**` to `instance:pipeline:run_id:**stage**` (both serialization in `Display` and deserialization in `FromStr`).

**ctx_rec_7** — Populated `instance` from `self.zbobr.config().instance` when constructing `StageInfo` in `zbobr-dispatcher/src/cli.rs`.

Also fixed two pre-existing issues uncovered while running tests:
- Missing `IssueUser` struct in `github.rs` (introduced in commit 43806f1)
- `roundtrip_preserves_context` test using position-based record lookup, which breaks because serialization reorders records (non-checkbox first)

All 45+39+9 tests pass.