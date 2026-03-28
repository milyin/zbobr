## GitHub backend setup: create `zbobr:<instance>` label; force-cleanup other instance labels

Modify `ZbobrTaskBackendGithubImpl::setup()` in `zbobr-task-backend-github/src/github.rs`.

**What to add:**
1. Define a constant `INSTANCE_LABEL_PREFIX: &str = "zbobr:";` alongside the existing `STATE_PREFIX` and `FLAG_LABEL_PREFIX` constants.
2. Compute the current instance label name: `format!("{}{}",  INSTANCE_LABEL_PREFIX, self.backend_config.instance)` (e.g. `"zbobr:default"`).
3. After creating state labels, create (or update on force) the instance label:
   - Use a distinctive color (e.g. purple/violet: `"6f42c1"`)
   - Description: `"Instance: <instance_name>"`
   - If label does not exist: create it
   - If label exists and `force=true`: update it
   - If label exists and not force: log and skip
4. **When `force=true`:** after creating/updating the current instance label, scan all existing labels and delete any that start with `INSTANCE_LABEL_PREFIX` but are NOT the current instance label. This cleans up orphaned instance labels from removed/renamed instances.
5. **When NOT force:** skip deletion entirely (leave other instance labels untouched).

**Why:** The label `zbobr:<instance>` is how users manually tag GitHub issues to route them to a specific dispatcher instance. The setup command creates the label so users can apply it. The `--force` cleanup removes stale labels from old instances that are no longer running.

**Pattern to follow:** The existing state label management code in `setup()` is the analog — see how it creates, updates (on force), and deletes obsolete `state:*` labels. Apply the same pattern to `zbobr:*` labels.