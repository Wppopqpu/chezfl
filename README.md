# chezfl

A personal system state manager inspired by Nix, written in Rust.
Users declare desired state (packages, repos, services) as **targets** and
define **tasks** that satisfy them — by writing Rust code.

## Philosophy

- **Configuration as Rust Code** — no YAML, no TOML, no DSL. You call
  chezfl's API directly in a Rust binary. See
  [ADR-0001](docs/adr/0001-configuration-as-code.md).
- **Declare, then converge** — describe the target state and the actions
  needed to reach it; chezfl figures out what to run and in what order.
- **Idempotent tasks** — tasks should be safe to run repeatedly.
- **Best-effort** — if a dependency can't be satisfied, dependent targets
  are skipped but the rest continues.

## Quickstart

### Add chezfl as a dependency

```toml
[dependencies]
chezfl = { git = "https://github.com/you/chezfl" }
anyhow = "1"
```

### Write your config (`src/main.rs`)

**Builder API:**

```rust
use chezfl::{App, Target, Task, run_cli};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    app.target(
        Target::new("rg_installed")
            .check(|| {
                Ok(std::process::Command::new("which")
                    .arg("rg")
                    .status()?
                    .success())
            })
            .depends_on("apt_ready"),
    );

    app.task(
        Task::new("install_rg")
            .satisfies("rg_installed")
            .depends_on("apt_ready")
            .label("install")
            .run(|| {
                // Idempotent — safe to run again
                std::process::Command::new("sudo")
                    .args(["pacman", "-S", "--noconfirm", "ripgrep"])
                    .status()?;
                Ok(())
            }),
    );

    app.target(Target::new("apt_ready")); // aggregate — satisfied when deps are

    // Parse CLI args and dispatch
    run_cli(&mut app)
}
```

**Macro API (identical semantics, less boilerplate):**

```rust
use chezfl::{target, task, run};

fn main() -> anyhow::Result<()> {
    target!("apt_ready");

    target!("rg_installed",
        check: || Ok(std::process::Command::new("which").arg("rg").status()?.success()),
        depends_on: [apt_ready],
    );

    task!("install_rg",
        satisfies: [rg_installed],
        depends_on: [apt_ready],
        labels: ["install"],
        run: || {
            std::process::Command::new("sudo")
                .args(["pacman", "-S", "--noconfirm", "ripgrep"])
                .status()?;
            Ok(())
        },
    );

    run!()
}
```

### Build and run

```bash
cargo build --release

# Show current state (no side effects)
./target/release/my-config check

# Show what *would* happen
./target/release/my-config plan

# Converge: check → run tasks → re-check
./target/release/my-config apply
```

## CLI Reference

```
Usage: my-config [COMMAND]

Commands:
  check   Check target satisfaction (no side effects)
  plan    Plan — simulate apply without running tasks
  apply   Apply — converge toward desired state

Flags (every command):
  --label <LABEL>         Only consider tasks with this label (repeatable)
  --exclude-label <LABEL> Exclude tasks with this label (repeatable)
  --set <NAME=bool>       Manually set target state (bypasses check, repeatable)
  --unset <NAME>          Remove stored state for a target (repeatable)
  --recheck <NAME>        Alias for --unset, forces re-check (repeatable)
```

### Examples

```bash
# Check all targets
./my-config check

# Check only specific targets (includes transitive deps)
./my-config check rg_installed

# Only run "install" labelled tasks
./my-config apply --label install

# Skip "system" labelled tasks
./my-config apply --exclude-label system

# Manually mark a target as satisfied
./my-config check --set docker_installed=true

# Force re-check
./my-config check --recheck docker_installed
```

## Domain Model

See [CONTEXT.md](CONTEXT.md) for the full glossary and design rationale.

### Targets

A **Target** is a concrete desired state. Two kinds:

- **Leaf target** — has a `check` closure that probes the real system
  (e.g. "is ripgrep installed?")
- **Aggregate target** — no check; satisfied when all its dependencies are
  satisfied. Useful for grouping.

Each target is satisfied by **exactly one** task. Targets form an
acyclic dependency DAG.

### Tasks

A **Task** is an actionable unit that satisfies 1+ targets.

- Has a `run` closure (idempotent, serial, stdin-forwarded)
- Declares labels for filtering
- Depends on *targets* (not other tasks) — this keeps the dependency model
  simple and avoids redundant execution
- No rollback

## Conventions

- `src/lib.rs` — library root; `src/main.rs` — CLI binary (thin, delegates to lib)
- Tests live in `tests/` (integration) and inline `#[cfg(test)] mod tests` (unit)
- `anyhow` for error handling; `thiserror` for library errors
- Public API goes through lib; main only parses CLI args and calls lib

## Development

```bash
cargo build               # debug build
cargo build --release     # release build
cargo test                # all tests
cargo test <name>         # single test
cargo fmt                 # format
cargo clippy -- -D warnings  # lint
```

## License

MIT
