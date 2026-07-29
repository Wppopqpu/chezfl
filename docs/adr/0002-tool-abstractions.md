# ADR-0002: Tool Abstractions

**Status:** Accepted
**Date:** 2026-07-29
**Deciders:** chezfl maintainer

## Context

Users define Task `run` closures that execute system commands (package install,
git clone, service management, etc.). These closures need a convenient way to
run commands, handle output, and deal with common failure modes. Without
standard abstractions, every config project would reimplement the same patterns.

## Decision

We introduce two layers:

### Layer 1 — `Cmd` (core command runner)

A builder-style command runner in `src/cmd.rs` that wraps `std::process::Command`.

**API surfaces:**

```rust
// Quick one-shot (functional)
pub fn run_cmd(program: &str, args: &[&str]) -> anyhow::Result<Output>;

// Builder (configurable)
pub fn cmd(program: &str) -> Cmd;

pub struct Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}
```

**Builder methods on `Cmd`:** `arg`, `args`, `env`, `dir`, `timeout(Duration)`,
`retry(usize)`.

**Execution modes (two separate APIs):**

| Method  | stdin | stdout/stderr   | Returns                          |
|---------|-------|-----------------|----------------------------------|
| `run()` | null  | captured (pipe) | `Output` with stdout/stderr      |
| `exec()`| inherit | inherit       | `Output` with status (empty strings for stdout/stderr) |

- **`run()`** is for silent commands where you want to inspect output.
- **`exec()`** is for interactive commands (sudo, pacman, yay) where the user
  needs to see output and respond to prompts in real time.

**Timeout:** A background thread sends SIGKILL after the duration expires.

**Retry:** The command is re-executed from scratch up to N times if it fails.

**Error handling:** Non-zero exit code → `Err(anyhow::Error)`. The error message
includes the command, args, exit code, and stderr (if captured). The `Output`
struct is only returned on success — callers that need output on failure should
capture it manually via `Cmd`.

### Layer 2 — Tool modules (convenience wrappers)

Thin modules under `src/tools/` that wrap `Cmd` for specific programs:

- `tools::git` — `clone`, `pull`, `fetch`, `status`, etc.
- `tools::yay` — `install`, `remove`, `update`, `is_installed`

Each function signature: `fn(...) -> anyhow::Result<Output>`.

Tools are **Task helpers**, not targets. They are designed to be called inside
a task's `run` closure. Users may also use `Cmd` directly for programs without
a dedicated module.

### Relationship to domain model

```
Task.run ──calls──> tool::git::clone(url) ──uses──> cmd("git").args([...]).exec()
              or
Task.run ──calls──> cmd("my-tool").args([...]).run()
```

Tools are **not** Targets. A Target describes *what* state is desired; a tool
is a *how* helper for the Task that achieves it.

## Consequences

- **Positive:** Users can write `git::clone(url)` or `yay::install(["rg"])`
  without boilerplate.
- **Positive:** Interactive vs captured execution is explicit at the call site.
- **Positive:** Timeout and retry are built-in, reducing error handling noise.
- **Positive:** Functional `run_cmd` covers 80% of cases; builder `cmd` covers
  the remaining 20% (env, dir, timeout, retry).
- **Negative:** Two APIs (run/exec) instead of one — users must choose.
- **Neutral:** Adding new tool modules is straightforward but requires a new
  file and a `pub mod` entry in `tools/mod.rs`.

## Alternatives considered

- **Single API with flag:** Rejected because interactive vs capture has
  fundamentally different semantics (piping vs inheriting) and combining them
  would make the return type and error handling awkward.
- **Trait-based Tool trait:** Rejected — too much abstraction for simple
  command wrappers. Free functions are simpler and compose naturally.
- **External crate:** Rejected — chezfl is in early development; built-in
  tools keep the dependency tree small and allow tight integration with the
  target/task model.
