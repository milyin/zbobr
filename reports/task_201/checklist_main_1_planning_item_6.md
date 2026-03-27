# Update setup() and remove signal_labels from TaskBackend trait

## 1. Update TaskBackend trait in zbobr-api/src/backend.rs

Change the `setup` method signature — remove `signal_labels` parameter:

```rust
// OLD:
async fn setup(&self, force: bool, signal_labels: &[String]) -> anyhow::Result<()>;

// NEW:
async fn setup(&self, force: bool) -> anyhow::Result<()>;
```

Also update the doc comment to remove mention of signal_labels.

## 2. Update setup() in zbobr-task-backend-github/src/github.rs (lines ~632–728)

Change signature to `async fn setup(&self, force: bool) -> anyhow::Result<()>`.

Remove the entire signal labels block (lines ~663–693):
- Delete `SIGNAL_LABEL_COLOR` constant
- Delete `existing_signal_labels` computation
- Delete the "delete obsolete signal labels" loop
- Delete the "create missing signal labels" loop

In the state labels section (lines ~696–721), remove pipeline label creation:
```rust
// OLD:
let state_labels: Vec<String> = ALL_STATE_LABEL_NAMES
    .iter()
    .map(|name| format!("{}{name}", STATE_PREFIX))
    .chain(
        [Pipeline::MAIN, Pipeline::MERGE]
            .iter()
            .map(|name| format!("{}{name}", PIPELINE_PREFIX)),
    )
    .collect();

// NEW:
let state_labels: Vec<String> = ALL_STATE_LABEL_NAMES
    .iter()
    .map(|name| format!("{}{name}", STATE_PREFIX))
    .collect();
```

Also remove `PIPELINE_PREFIX` constant (line ~19) if no longer used anywhere else.

## 3. Update the GithubTaskBackend wrapper in zbobr-task-backend-github/src/github.rs (line ~1471)

```rust
async fn setup(&self, force: bool) -> anyhow::Result<()> {
    self.inner.setup(force).await
}
```

## 4. Update zbobr-task-backend-fs/src/fs.rs

Both `setup()` implementations (lines ~577 and ~691) — remove `signal_labels` parameter:

```rust
async fn setup(&self, _force: bool) -> anyhow::Result<()> { Ok(()) }
// and:
async fn setup(&self, force: bool) -> anyhow::Result<()> {
    self.inner.setup(force).await
}
```

## 5. Update zbobr-dispatcher/src/lib.rs setup_repository() (lines ~187–215)

Remove signal_labels construction and just call:
```rust
self.task_backend.setup(force).await
```

Delete these lines:
```rust
let mut signal_labels: Vec<String> = Vec::new();
signal_labels.push(format!("{SIGNAL_PREFIX}go_{stage_name}"));
// ...
signal_labels.push(format!("{SIGNAL_PREFIX}return"));
signal_labels.push(format!("{SIGNAL_PREFIX}return_failure"));
signal_labels.sort();
signal_labels.dedup();
self.task_backend.setup(force, &signal_labels).await
```

Also remove the `use` import for `SIGNAL_PREFIX` if it's only used here.

## 6. Update test mock impls

**zbobr-dispatcher/src/backend.rs** (line ~27):
```rust
async fn setup(&self, _force: bool) -> anyhow::Result<()> { Ok(()) }
```

**zbobr-dispatcher/src/task.rs** (line ~918):
```rust
async fn setup(&self, _force: bool) -> anyhow::Result<()> { Ok(()) }
```

**Why:** Signal labels are gone; setup no longer needs to create them. Pipeline labels are gone too. Only state:* and flag:* labels remain.