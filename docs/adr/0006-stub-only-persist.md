# ADR-0006: Only Stub Target State Persisted to Disk

**Status:** Accepted
**Date:** 2026-07-30
**Deciders:** chezfl maintainer
**Supersedes:** ADR-0005 (the early-save timing is kept, but the scope is narrowed from "all targets" to "stubs only")

## Context

ADR-0005 introduced immediate persistence of all manual overrides to prevent stub targets from losing their `--set` state on crash or `plan`. At the time, saving for all targets was chosen because "saving for all targets is equally correct and simpler."

However, persisting leaf and aggregate target state carries ongoing costs:

1. **Cross-run caching of leaf check results.** A leaf target whose `satisfied = true` was persisted would skip its check on the next invocation — even if the real system state had changed (e.g., a package was uninstalled externally). This produced stale results until an explicit `--recheck`.

2. **State file clutter.** Every distinct target visited by `run_check` or `run_apply` wrote an entry to `state.toml`. Over time — especially with generated targets (e.g., looping over repos) — the state file grew large and opaque.

3. **Incorrect mental model.** Users expected `check` to *check* — to probe the real system and report truth. The caching mechanism silently violated that expectation by returning stale data. Manual overrides (`--set`) are the explicit way to bypass checks; caching should not be a second, implicit bypass.

## Decision

Only stub targets (those with neither a `check` function nor `depends_on`) have their state persisted to disk. Leaf and aggregate target state exists only in memory during the current run.

**In-memory caching is preserved** — within a single `run_apply` invocation, a leaf target that was just checked and found satisfied will not be re-checked for subsequent dependents or task re-checks. But on the next invocation, every leaf check runs fresh.

The implementation:

- `App::save_state()` filters `State` entries to only those belonging to registered stub targets before writing.
- `set_check_result()` continues to write all targets to in-memory state (supporting in-memory caching).
- Direct `self.state.save_to(path)` calls in `run_check` and `run_apply` are replaced with `self.save_state()` so the filtering is applied automatically.
- `run_cli`'s immediate-save path already calls `app.save_state()`; no change needed there.

## Consequences

- **Positive:** Every `check` invocation re-probes the real system. No stale cache.
- **Positive:** State file contains only stub entries — small, predictable, human-readable.
- **Positive:** The mental model is clear: if a target has a check, it always runs; if you want to bypass it, use `--set`.
- **Negative:** Leaf checks run on every invocation, which is slightly slower. In practice this is negligible (most checks are `is_file`/`is_runnable`).
- **Negative:** In-memory caching only spans one CLI invocation. A long-running process (not chezfl's model anyway) would re-check repeatedly.
- **Neutral:** ADR-0005's rejection of "save only for stubs" is now superseded. The earlier rationale (simpler to save for all) was correct at the time but didn't weigh the stale-cache cost heavily enough.

## Alternatives considered

- **Save all targets (status quo before this ADR):** Simpler code, but stale cache and state file bloat. Rejected.

- **Persist everything but disable cross-run caching:** Always re-run checks even when `satisfied = true` is found in state. This is what 0006 effectively achieves — by not persisting leaf state at all, we naturally avoid caching. The stub-only approach is cleaner because it keeps the state file minimal.
