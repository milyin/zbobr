# Dispatcher Task State Transitions

This document explains the logic the **dispatcher** uses to manage task state transitions. It is accompanied by a Graphviz `.dot` file (`transitions.dot`) illustrating the flow.

## States
A task can be in one of the following states:

- `PENDING`
- `PREPARING`
- `PLANNING`
- `WORKING`
- `REVIEWING`
- `MERGING`
- `DONE`

## Signals and Flavors
Transitions between states are *only* triggered by two pieces of immutable information attached to the task:

1. **Signal:** a string of the form `go_<STATE>` for each state except `PENDING`.
2. **Flavors:** a set of on/off flags. Currently there are two flavors:
   - `conflict`
   - `pause`

The dispatcher has **write access** only to the task's state. It may **read** the signal and the set of flavors but cannot modify them. Role sessions (see below) have no read access to state, signals, or flavors, but they may set signals and flavors blindly.

## Base Transition Rules

1. When the dispatcher inspects a task, it looks at its current state and flavors.
2. **If the state is `PENDING` and no flavor overrides apply**, it interprets the `signal`:
   - `go_PREPARING` moves the task to `PREPARING` and starts the corresponding role session.
   - `go_PLANNING` moves to `PLANNING`, and so on for the other states.
3. A task in any non-`PENDING` state is only returned to `PENDING` when the role session for that state finishes.
4. There are **no direct transitions** between non-`PENDING` states.

> In other words: `PENDING <-> X` is the only transition pattern, with `X` being another state.

## Flavor Overrides

Flavors may modify the normal behaviour:

- **`conflict`**: If a task in `PENDING` has the `conflict` flavor set, the dispatcher ignores the current signal completely. Instead, it sets the state to `MERGING` and starts the merging role session.

- **`pause`**: If `pause` is active, the dispatcher ignores signals and leaves the task in `PENDING` indefinitely. No role session is started.

Flavors are evaluated each time the dispatcher processes the task; they do not persist into the role session logic except for later detail.

## Role Sessions

Each state (except `PENDING`) corresponds to a *role session*. When the dispatcher moves a task into that state it launches the associated session and then returns the task to `PENDING` when the session ends. Role sessions:

- Have **no visibility** into the task’s state, signal, or flavors.
- May *blindly* set new signals and adjust flavors for future dispatching cycles.

The specifics of how role sessions alter signals/flavors will be documented separately.

---

Additional context is available in the DOT file for visualization. Use Graphviz tools (e.g., `dot -Tpng transitions.dot -o transitions.png`) to render the state diagram.