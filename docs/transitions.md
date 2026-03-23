# Dispatcher Task State Transitions

<!-- markdownlint-disable MD047 -->

This page describes how the dispatcher drives tasks through its lifecycle. A companion DOT graph (`transitions.dot`) lays out the same logic visually.

## Concepts

* **task** – the entity processed by the dispatcher.

* **role session** – an operation on a task by an agent in some role defined by a prompt. The result of a session is a change to the task's signal and/or flags. The dispatcher routes the task to the next step depending on this data.

* **work branch** – the Git branch in the repository in the directory provided by the dispatcher where the dispatcher expects to see the results of the agent's work.

* **destination branch** – the Git branch in the repository in the directory provided by the dispatcher where the original sources are stored.

## Task fields

* **state** – the current state of the task. A task in the `PENDING` state is handled by the dispatcher. Tasks in other states are handled by the corresponding session (`MERGING`, `PREPARING`, `PLANNING`, `WORKING`, and `REVIEWING`) executed by the dispatcher. After processing, a task always returns to the `PENDING` state. A task set to `DONE` state is not handled further.

* **signal** – an enum value indicating the desired next state for the task.

* **conflict** – boolean flag indicating a merging conflict.

* **pause** – boolean flag temporarily blocking task processing.
* **confirm** – boolean flag indicating that every stage transition should also set `pause`.  When true the dispatcher (and CLI updates) will automatically pause the task whenever its stage changes, giving operators an opportunity to review before additional work proceeds.

## Task processing

The [transitions.dot] file is the source of truth. Here are only additional explanations.

Eligible tasks are processed at regular time intervals. An eligible task is a task in the pending state with a signal set and with the pause flag not set.

Depending on the signal and flags, the dispatcher decides which agent role session to start for the task.

Each session has its own possible outcomes which are determined by the MCP operation available to the role and by the session’s post-processing logic. This document only lists these outcomes without going into specific rules.

## Stage transition fields

Each `StageDefinition` has two optional transition fields:

* **`on_success`** — stage name to transition to when the agent calls `report_success`. If `None`, defaults to the next sequential stage in the pipeline, or `return` if this is the last stage.
* **`on_failure`** — stage name to transition to when the agent calls `report_failure`. If `None`, defaults to `return_failure` (exit pipeline).

Signal values follow the same format as before: `go_<stage>` to transfer within the current mode, `call_<mode>` to invoke a sub-mode, or `return` to finish the current mode.