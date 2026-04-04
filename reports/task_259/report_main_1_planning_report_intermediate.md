# Plan: Systematize and Reduce Labels

## Context

The current system changes `state:*` labels on every state transition (Ready, Pending, Running, etc.), generating excessive noise in the GitHub comments feed. The goal is to reduce label usage to a few meaningful signals (`zb:pause`, `zb:done`, `zb:assigned`) and store state as an explicit parameter in the issue body instead of deriving it from labels. The instance label is also renamed from `zbobr:<instance>` to `zb:node:<instance>`.

## Algorithm Flaw

The task description specifies:
- **State processing**: "state is Pause -> set `zb:pause` label if not set"
- **Expected workflow**: "task already in the Pause state but without `zb:pause` label (i.e. unpaused by user) switched to Ready state"

These contradict each other. If state=Pause and zb:pause is missing, the algorithm says RE-ADD the label, but the expected workflow says UNPAUSE.

**Proposed fix**: Split the Pause state processing into two cases:
- state is Pause AND `zb:pause` IS set → no action (stay paused)
- state is Pause AND `zb:pause` is NOT set → pop stack, restore previous state (unpause)

The "set zb:pause when pausing" happens during label reconciliation after any state transition to Pause (either internal or user-initiated via label).

### Edge case: `zb:pause` on a Done task

The label processing rule "zb:pause is set, state is not Pause → push state, set Pause" would fire even for Done tasks, allowing a user to "un-done" a task by adding zb:pause. This seems unintentional. Propose: skip label-initiated pause if state is Done.

## Design

### New field: `pause_label: bool` on Task struct

The Task struct needs to carry whether the `zb:pause` label is present on the GitHub issue. This is separate from the existing `task.pause` flag (internal pause request stored as parameter). The GitHub backend sets this from labels; the FS backend always sets it to `false`.

This separation is needed because:
- Internal pause (`task.pause`): zbobr code requests pause, consumed by `apply_pause_to_state`
- Label pause (`task.pause_label`): user added `zb:pause` label, detected by dispatcher
- Label unpause: user removed `zb:pause` while state=Pause, detected by dispatcher

### State storage in parameters

Add `PARAM_STATE = "state"`. State is serialized/deserialized using the existing `State::to_serde_string()` / `State::from_serde_string()` format (e.g. `"ready"`, `"pending:main"`, `"running:main:working"`).

### Label reconciliation (replaces `apply_state_change`)

After saving task params in `modify_task_internal`, reconcile labels based on state:
- **Pause** → ensure `zb:pause` is set (don't touch `zb:assigned`)
- **Done** → ensure `zb:done` is set, remove `zb:pause` and `zb:assigned`
- **Other non-Empty** → remove `zb:pause`, ensure `zb:assigned` is set
- **Empty** → no label changes

### Dispatcher loop label/state processing

In the dispatcher loop, before `resolve_next_action`:

1. **New task detection**: state=Empty AND no zb:pause/zb:done/zb:assigned labels → set state to Ready
2. **User-initiated pause**: `task.pause_label` AND state != Pause AND state != Done → push stack, set Pause
3. **Internal pause**: `task.pause` flag (existing `apply_pause_to_state`)
4. **User-initiated unpause**: state=Pause AND !`task.pause_label` AND !`task.pause` → pop stack, restore previous state

## Changes by module

### 1. zbobr-api/src/task.rs
- Add `pause_label: bool` field to Task struct (default `false`)
- Ensure `State` serde methods are public (for param storage)

### 2. zbobr-task-backend-github/src/github.rs
- Rename `INSTANCE_LABEL_PREFIX` from `"zbobr:"` to `"zb:node:"`
- Add `PARAM_STATE` constant
- Replace `STATE_PREFIX`, `STATE_LABEL_*`, `ALL_STATE_LABEL_NAMES` with new constants: `ZB_LABEL_PAUSE = "zb:pause"`, `ZB_LABEL_DONE = "zb:done"`, `ZB_LABEL_ASSIGNED = "zb:assigned"`
- `issue_to_task`: read state from `PARAM_STATE` param (not labels); set `pause_label` from zb:pause label presence
- `task_to_string_params`: write state via `PARAM_STATE`
- Remove `labels_to_state` and `state_to_labels`
- Replace `apply_state_change` with new `reconcile_labels`: set/remove zb:pause, zb:done, zb:assigned based on state
- `setup`: create zb:pause, zb:done, zb:assigned labels; clean up old state:* labels; rename zbobr:* to zb:node:*
- Replace `state_label_color` with colors for new labels
- Remove `FLAG_LABEL_PREFIX` (legacy flag: cleanup no longer needed)

### 3. zbobr-task-backend-github/src/config.rs
- Update comment about instance label format

### 4. zbobr-task-backend-fs/src/fs.rs
- Set `pause_label: false` in Task construction

### 5. zbobr-dispatcher/src/cli.rs
- Add new-task detection logic (Empty + no labels → Ready)
- Add user-initiated pause logic (pause_label + state != Pause)
- Add unpause logic (state=Pause + !pause_label → pop stack, restore state)
- Keep existing `apply_pause_to_state` for internal pause (task.pause flag)

### 6. Tests
- Update integration tests that check labels or state transitions
- Update any test helpers that create tasks with specific labels

## Verification

1. `cargo build` – ensure compilation
2. `cargo test` – run all tests
3. `cargo clippy` – lint check
4. Manual verification of expected workflows:
   - New task gets Ready state + zb:assigned label
   - State transitions don't change labels (only zb:assigned stays)
   - Internal pause sets zb:pause
   - User adding zb:pause triggers pause
   - User removing zb:pause triggers unpause
   - Done sets zb:done, removes zb:pause/zb:assigned