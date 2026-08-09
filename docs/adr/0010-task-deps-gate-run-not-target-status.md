# ADR-0010: Task Dependencies Gate the Run, Not the Target's Status

**Status:** Accepted
**Date:** 2026-08-09
**Deciders:** chezfl maintainer

## Context

A leaf target's satisfaction is (and always was) decided by its own `check` function and check deps — not by whether its satisfying task is available to run. But `run_apply`/`run_plan` reported a target as unsatisfied with the detail `"(task deps not satisfied)"` whenever its task's `depends_on` were unsatisfied, and additionally inserted all targets satisfied by that task into the `blocked` set, cascading "(skipped, upstream failure)" to downstream targets. This conflated two distinct concerns: whether the target's desired state holds, and whether the task that would restore it may run. In practice it meant a stub prerequisite like `ssh_key` (always unsatisfied unless `--set`) permanently gated and misreported an otherwise-independent target.

## Decision

A task's `depends_on` gates **only when the task runs**, never the satisfaction status of the targets it satisfies:

- When a target's check reports unsatisfied but its satisfying task has unsatisfied deps, the task does **not** run and the target is reported unsatisfied **with its own check-derived detail** — never `"(task deps not satisfied)"`.
- An unsatisfied task dependency does **not** cascade: nothing is added to the `blocked` set, and downstream targets are evaluated on their own merits.
- The `run_plan` path mirrors `run_apply`; its now-dead `blocked` machinery was removed.

A task still never runs until its deps report satisfied; `--set ssh_key` remains necessary for the clone task to actually execute — it just no longer distorts the target's reported status or blocks unrelated targets.

## Consequences

- **Positive:** Target status is now a pure function of the target's own declaration (check + check_dep + manual override), independent of task availability.
- **Positive:** A stub prerequisite no longer cascades blocks through the graph.
- **Neutral:** A task that never gets its deps satisfied simply stays unexecuted — the target honestly reports unsatisfied until the user intervenes (e.g. `--set` the prerequisite).
- **Negative:** The user sees a target as unsatisfied without a hint that its task was skipped for lack of deps; the reason must be found by reading the dependency declarations.

## Considered Options

1. **Status quo** — target marked unsatisfied `"(task deps not satisfied)"` and satisfied-by targets blocked. Rejected: conflates run-gating with target status; cascades blocks; contradicts the leaf definition that status is decided by the check.

2. **Gate the run, keep the target's own status (chosen)** — `depends_on` remains a run gate; the target reports exactly what its check says; no cascade.

3. **Run the task anyway even with unsatisfied deps** — `depends_on` becomes a pure ordering hint. Rejected: a task whose prerequisites (e.g. ssh key, package install) are unmet is likely to fail; running it anyway is pointless and potentially destructive.

4. **Remove task `depends_on` entirely** — target-level `check_dep` already expresses prerequisites. Rejected: task-level deps remain useful for *execution* prerequisites that should not demote the target (e.g. `ssh_key` must be set for clone to work, but the repo target's status should not be blamed for it).
