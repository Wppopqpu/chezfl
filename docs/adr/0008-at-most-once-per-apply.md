# ADR-0008: Tasks Run At Most Once per Apply

**Status:** Accepted
**Date:** 2026-08-09
**Deciders:** chezfl maintainer

## Context

A task may satisfy multiple targets. `run_apply` walks targets in topological order and runs a target's task when the target is unsatisfied. Previously, if a task ran for an earlier target but another target it satisfies was still unsatisfied after the re-check, the task ran *again* when that second target was reached — effectively presuming that every task is safe to run repeatedly.

The documentation codified this presumption ("a task should be idempotent, designed to be run repeatedly"). That puts an unrealistic burden on task authors: many real actions are one-shot (`git clone`, systemd unit install, repo bootstrap) where a second run fails or is destructive. chezfl should not *presume* re-runnability; it should guarantee it never runs a task twice in one invocation.

## Decision

Each task executes **at most once per `apply` invocation**.

- The apply engine tracks which tasks have already run. When an unsatisfied target's satisfying task already ran this invocation, the target is left unsatisfied and reported as "(task already ran, target still unsatisfied)" — the task is not run again.
- Tasks are **not presumed idempotent or re-runnable**. Within a single run, double execution is impossible by construction.
- Across invocations, the `check` function remains the guard against redundant runs: a fresh `apply` re-runs a task only for targets its check reports as unsatisfied.

## Consequences

- **Positive:** One-shot, non-idempotent tasks are safe — no destructive double-run within a single apply.
- **Positive:** Documentation no longer requires tasks to be idempotent.
- **Neutral:** A multi-target task that leaves one of its targets unsatisfied requires user intervention (a `check` that eventually reads true) before a later apply re-attempts it.
- **Note:** This guarantee is scoped to a single invocation. Cross-invocation behavior is unchanged.

## Considered Options

1. **Presume idempotency (status quo)** — allow a task to run multiple times per apply. Rejected: destructive or erroneous double-run for one-shot actions; contradicts the "not presumed re-runnable" stance.

2. **Guarantee at-most-once per invocation via an executed-task set** — chosen. Minimal, matches the model: tasks already run are recorded and skipped.

3. **Persist cross-invocation "attempted" markers** to also prevent re-runs on later invocations. Deferred — out of scope. Across runs, the `check` function is the guard, and requiring manual state for every unsatisfied target would break convergence semantics (see ADR-0006).
