# ADR-0003: Interactive Tools by Default

**Status:** Accepted
**Date:** 2026-07-29
**Deciders:** chezfl maintainer

## Context

Built-in tool wrappers (`yay::install`, `yay::remove`, etc.) run system
commands. These commands modify the system (install/remove packages).
They must choose: run silently with `--noconfirm` (no prompts), or run
interactively (inherit terminal, show prompts, wait for user input).

The initial implementation used `--noconfirm` for all operations, assuming
the user would review the plan beforehand and trust the automation.

## Decision

All built-in tool wrappers that modify the system **must not** pass
`--noconfirm` or equivalent auto-confirm flags. They use [`exec()`] —
interactive mode — making the user confirm every destructive operation
at the terminal.

The only exceptions are read-only queries (`yay -Qi`, `git status`, etc.)
which use [`run()`] (captured, non-interactive).

[`exec()`]: https://docs.rs/chezfl/latest/chezfl/cmd/struct.Cmd.html#method.exec
[`run()`]: https://docs.rs/chezfl/latest/chezfl/cmd/struct.Cmd.html#method.run

## Rationale

- **Safety**: chezfl runs tasks automatically and serially. An unexpected
  package conflict or version change could cause damage without the user
  noticing. Requiring confirmation adds a human checkpoint.
- **Idempotence mismatch**: `--noconfirm` is useful for CI/CD pipelines
  where there is no human in the loop. chezfl is a personal tool — the
  human is present at the terminal.
- **Tool philosophy**: chezfl aims to *converge* toward desired state, not
  to be an unattended provisioning system. Interactive confirmation aligns
  with that philosophy.

## Consequences

- **Positive:** Users see every package operation before it happens and can
  abort with Ctrl-C.
- **Positive:** No risk of `--noconfirm` automatically answering "yes" to
  prompts the user would have declined (e.g., replacing a config file).
- **Negative:** Cannot run chezfl unattended (e.g., from cron) for
  operations that require confirmation.
- **Neutral:** Users who want silent automation can still use the
  [`Cmd`] builder directly with their own flags.

[`Cmd`]: https://docs.rs/chezfl/latest/chezfl/cmd/struct.Cmd.html

## Alternatives considered

- **Flag-controlled**: Add a `confirm: bool` parameter to each tool
  function. Rejected because it adds complexity to every call site and
  encourages unattended use, which is not a chezfl goal.
- **Per-tool environment variable**: Let `CHEZFL_CONFIRM=0` skip prompts.
  Rejected — too implicit and hard to discover.
