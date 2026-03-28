In `github.rs`, the `configure_repo` method currently:
1. Creates/updates `flag:pause` and `flag:confirm` labels on GitHub
2. Includes those flag labels in the "expected labels" set
3. Checks `FLAG_PREFIX` when deleting obsolete labels

Remove all three of these — since flags are no longer stored as labels, there are no flag labels to manage. The expected-labels set and obsolete-label deletion logic should only operate on `state:*` labels going forward.