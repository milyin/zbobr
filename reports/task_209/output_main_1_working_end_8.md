Both correctness gaps are fixed:

1. **Retry loop** (`cli.rs`): `CliStageRunner::run()` now wraps provider selection + execution in a `loop`. When `connectivity_failure` is true, the failing provider is excluded and the loop immediately retries with the next available provider. `select_provider()` returning an error (all providers exhausted) terminates the loop naturally.

2. **Global tool validation** (`config.rs`): Removed the `!self.tools.is_empty() &&` guard — `validate()` now always requires `dispatcher.tool` to exist in `[tools]`, rejecting empty-tools configs at startup.