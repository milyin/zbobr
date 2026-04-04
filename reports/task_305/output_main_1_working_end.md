All done. The implementation is complete with two commits:

1. **commands.rs** — appends `/{instance}` to `workspaces` and `repos_dir` at the wiring point before backends are constructed
2. **env.rs** — mirrors the same instance-scoping in both test setup functions (`init_fs_fs` and `init_github_github`)

All tests and clippy pass cleanly.