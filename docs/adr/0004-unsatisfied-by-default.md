# ADR-0004: Targets Without Check or Deps Are Unsatisfied by Default

**Status:** Accepted
**Date:** 2026-07-30
**Deciders:** chezfl maintainer

## Context

A target with no `check` function and no `depends_on` declarations is an edge case — it has no way to determine satisfaction. There are two valid interpretations:

1. **Vacuous truth**: all zero of its dependencies are trivially satisfied → mark it satisfied. This creates a convenient "already done" default for targets the user hasn't wired up yet.
2. **Unsatisfied by default**: a target with no probe and no dependencies cannot be considered satisfied. The user must either add a `check`, add `depends_on`, or define a `Task` that `satisfies` it.

The initial implementation chose option 1 (vacuous truth). Experience showed this was surprising: declaring `target!("net")` as a simple namespace placeholder would silently show as satisfied, masking the fact that nothing had actually verified network connectivity. Users expected a bare declaration to appear as "not done" until explicitly configured.

## Decision

A target with no `check` function and an empty `depends_on` list is **Unsatisfied**. Its detail message is `"(no check, no deps)"`.

This affects:

- **Aggregate targets** — if an aggregate declares zero dependencies, it is unsatisfied. Only aggregates with at least one dependency derive their satisfaction from those deps.
- **Stub targets** — a target with neither `check` nor `depends_on` is explicitly a placeholder that needs a task or deps to become satisfiable.

Leaf targets (those with a `check`) are unaffected — they always run their check function and report the result regardless of deps.

## Consequences

- **Positive:** A bare `target!("net")` shows as Unsatisfied, signalling to the user that this target needs configuration.
- **Positive:** No silent auto-satisfaction for unconfigured targets — fewer surprises.
- **Positive:** The `is_empty` guard is a two-line check; zero complexity cost.
- **Negative:** A pure aggregation target with no deps is impossible (must have at least one dep or a check; otherwise it's a stub).
- **Neutral:** Existing aggregate chains (dep → dep → leaf-check) are unaffected because each aggregate already has at least one dep.

## Alternatives considered

- **Vacuous truth (rejected):** Convenient for chains but produces false-positive satisfaction for the most common onboarding pattern (a single bare-named target).
- **Compile-time validation (rejected):** Make it a validation error to have no check and no deps. Too restrictive — stubs are useful as forward declarations for tasks yet to be written.
