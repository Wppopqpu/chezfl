# ADR-0005: Immediate Persistence of Manual Overrides

**Status:** Superseded by ADR-0006 (early-save timing kept; scope narrowed to stubs only)
**Date:** 2026-07-30
**Deciders:** chezfl maintainer

## Context

`--set`, `--unset`, and `--recheck` flags let users manually override target
satisfaction state. Initially these overrides were held only in memory and
only persisted to disk later when `run_check` or `run_apply` explicitly saved.
The code contained a comment: *"State is saved during apply, or we can
provide a --save flag later"*.

This deferred approach had a practical problem for **stub targets** (targets
with no `check` function and no `depends_on`):

1. `--set stub` modifies in-memory state.
2. User runs `plan` (which never saves), or the process crashes during
   `apply`/`check` before the save at the end.
3. The manual override is lost on the next invocation.

Non-stub targets were unaffected because their `check` function or the
`set_check_result` call during `run_check`/`run_apply` would eventually
persist their state anyway. But stubs have no check — the only way their
state is ever persisted is via the deferred save.

## Decision

After all `--set`, `--unset`, and `--recheck` flags are processed,
immediately persist the state to disk if a state path is configured.

This is a single batch write after all manual modifications, not per-flag.
The save covers all targets, not just stubs, even though the behavioral
difference is only visible for stubs (non-stubs would be overwritten by
their check result during apply/check anyway).

The `App` struct gains a public `save_state()` method so the persistence
logic is reusable beyond `run_cli`.

## Consequences

- **Positive:** A `--set stub` followed by `plan` (or a crash) no longer
  loses the override.
- **Positive:** The save happens exactly once per CLI invocation, same as
  before (just earlier in the flow).
- **Positive:** `save_state()` is available for other call sites that may
  need to persist manually.
- **Negative:** `run_apply` and `run_check` still do a second save at the
  end of their execution. This is benign double-write — idempotent and
  cheap for a TOML file of typical size (< 1 KB). Could be optimized later
  with a dirty flag.
- **Neutral:** The `plan` subcommand now has a side effect (persisting
  manual overrides) where it had none before. This is acceptable because
  the side effect is limited to the manual-override path and does not
  affect the simulation logic.

## Alternatives considered

- **Save only for stubs (rejected):** Requires checking which targets are
  stubs before deciding to save. More complex code for no practical gain —
  saving for all targets is equally correct and simpler.

- **Per-flag immediate save (rejected):** Each `--set`/`--unset` writes
  the file independently. Multiple overrides in one invocation cause N
  writes instead of 1. No benefit over batch save.

- **Dirty flag with late save (deferred):** Keep the original approach:
  only save during `run_check`/`run_apply`. This was the status quo and
  loses stub overrides on `plan` or crash.

- **--save flag (deferred):** The original comment suggested a separate
  flag. Rejected because automatic save is simpler and less surprising —
  users already expect `--set` to "take effect."
