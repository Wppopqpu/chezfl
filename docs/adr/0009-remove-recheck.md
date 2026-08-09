# ADR-0009: Remove the `--recheck` Flag

**Status:** Accepted
**Date:** 2026-08-09
**Deciders:** chezfl maintainer

## Context

`--recheck <target>` was documented as "alias for `--unset`, forces re-check". In code it was a literal alias — `run_cli` processed it identically to `--unset` (both remove the target's stored state). The flag was meaningful before ADR-0006, when leaf results were persisted across runs and cached; `--recheck` invalidated that stale cache. After ADR-0006 made leaf and aggregate state memory-only, every invocation re-checks leaf targets fresh, so there is no cross-run cache left to invalidate. For stub targets (the only ones persisted), a "re-check" is meaningless — stubs have no check function, and their `--set` overrides are already cleared by `--unset`.

## Decision

Remove the `--recheck` flag. The CLI keeps `--set` (manual override) and `--unset` (clear stored state). At the same time, `--set` now honors its documented `<NAME=bool>` form: `--set <target>` pins `true`, `--set <target=bool>` pins the given value (previously the value was parsed but ignored).

## Consequences

- **Positive:** No misleading flag — "force re-check" was already the default behavior for leaf targets.
- **Positive:** `--set` behaves as documented (`<NAME=bool>`), letting users pin a target unsatisfied too.
- **Negative:** Shell scripts using `--recheck` need to switch to `--unset`. The two were always identical in behavior.

## Considered Options

1. **Keep `--recheck` as an alias.** Rejected: it advertises a capability that no longer exists, and `--unset` already names the real operation.

2. **Repurpose `--recheck` for a future cross-run "attempted" marker** (see the rejected option in ADR-0008). Rejected: no such marker exists today; keeping a vestigial flag on the off-chance is speculative.
