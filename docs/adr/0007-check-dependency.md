# ADR-0007: Check Dependency for Leaf Targets

**Status:** Accepted
**Date:** 2026-07-30
**Deciders:** chezfl maintainer

## Context

Leaf targets have a `check` function that probes the real system. Some checks are meaningless unless a prerequisite condition is met — e.g., checking whether ripgrep has a config file is meaningless unless the network is up (rg may hang trying to update itself) or unless ripgrep itself is installed. Previously this had to be handled inside the check function itself (check both preconditions and return `false` if they fail), or by wrapping the check in an aggregate target — neither of which made the dependency explicit in the model.

## Decision

Introduce **check dependency** (`check_dep`) as a new orthogonal dimension alongside `depends_on`:

- Only leaf targets may declare check deps (aggregate targets have no check to guard).
- All check deps must be satisfied before the check function executes (AND semantics).
- If any check dep is unsatisfied, the leaf target is **demoted to stub**: treated as unsatisfied, check skipped, no task runs for it.
- A task may still declare `satisfies` for a target with check deps, but will never execute for it when the target is in stub state.
- Check dep state is part of the in-memory cache key: a cached "satisfied" result is invalidated when a check dep becomes unsatisfied.

The dependency graph (combining both `depends_on` and `check_dep`) must remain acyclic. The two dimensions are otherwise independent: `depends_on` controls topological ordering and aggregate derivation; `check_dep` controls whether a leaf's check runs.

## Consequences

- **Positive:** The model is more expressive — prerequisites are declared explicitly in the target model rather than hidden inside check closures.
- **Positive:** Check deps participate in the caching protocol automatically (no manual cache management).
- **Positive:** The "no task for stubs" rule generalizes naturally — a leaf demoted by check dep behaves identically to a leaf whose check returned `false`.

## Considered Options

1. **Handle preconditions inside `check` closure** — the function could return `false` when a precondition fails. Rejected because the dependency is invisible to the model (no caching, no topological enforcement, no display).

2. **Use `depends_on` + aggregate for precondition** — wrap the real check in an aggregate. Rejected because aggregate targets derive satisfaction from deps, which is the wrong semantics: the leaf should still have its own independent check that runs when the precondition is met, not derive its state from the precondition.

3. **Make check deps available to all target kinds** — rejected because aggregate and stub targets have no check to guard.

4. **Allow task execution for demoted targets** — rejected by the "no task for stubs" rule, which keeps the model clean: a stub (original or demoted) is a target whose state cannot be changed by automation, only by manual `--set`.
