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

### Clone the repo and create a config branch

```bash
git clone https://github.com/wppopqpu/chezfl
cd chezfl
git checkout -b my-config        # your personal config branch
```

### Write your config (`src/bin/my_config.rs`)

A ready-to-use template is provided at [`src/bin/my_config.rs`](src/bin/my_config.rs).
Uncomment and fill in your own targets and tasks:

```rust
use chezfl::{App, Target, Task, run_cli};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    app.target(Target::new("network"));

    app.target(
        Target::new("rg_installed")
            .check(|| chezfl::tools::yay::is_installed("ripgrep"))
            .depends_on("network"),
    );

    app.task(
        Task::new("install_rg")
            .satisfies("rg_installed")
            .depends_on("network")
            .label("install")
            .run(|| {
                chezfl::tools::yay::install(&["ripgrep"])?;
                Ok(())
            }),
    );

    run_cli(&mut app)
}
```

**Macro API** (identical semantics, less boilerplate):

```rust
use chezfl::{target, task, run};

fn main() -> anyhow::Result<()> {
    target!("network");

    target!("rg_installed",
        description: "ripgrep (rg) is installed",
        check: || chezfl::tools::yay::is_installed("ripgrep"),
        depends_on: [network],
    );

    task!("install_rg",
        description: "Install ripgrep via yay",
        satisfies: [rg_installed],
        depends_on: [network],
        labels: ["install"],
        run: || {
            chezfl::tools::yay::install(&["ripgrep"])?;
            Ok(())
        },
    );

    run!()
}
```

See [`examples/laptop.rs`](examples/laptop.rs) for the full macro-API example.

### Build and run

```bash
# Default binary is my_config (set in Cargo.toml)
cargo run -- check          # check all targets
cargo run -- plan           # plan (dry-run)
cargo run                   # apply (default subcommand)

# Or build once and use the binary directly
cargo build --release
./target/release/my_config check
```

## Workflow: keep your config in sync with chezfl

chezfl and your config live in the same repository. A two-branch workflow
keeps them separate:

```text
main          — chezfl library source (receives upstream updates)
my-config     — your branch: src/bin/my_config.rs + chezfl source
```

**When chezfl has upstream changes:**

```bash
git checkout main
git pull origin main            # get latest chezfl
git checkout my-config
git merge main                  # bring chezfl updates into your config
```

**Your personal config stays in `my-config`** — the `main` branch is never
polluted with your local targets, so `git merge main` is always clean on
the config side.

This approach works because `src/bin/my_config.rs` is gitignored from the
upstream perspective — it only exists in your branch. The chezfl library
sources (`src/lib.rs`, `src/app.rs`, …) are the same in both branches.

## Cmd API (running commands)

chezfl provides [`Cmd`](https://docs.rs/chezfl/latest/chezfl/cmd/struct.Cmd.html)
for running external programs — a wrapper around `std::process::Command` with
timeout and retry support.

Two execution modes:

| Method | stdin | stdout/stderr | Use case |
|--------|-------|---------------|----------|
| `run()` | null | captured | Check if a program is installed, read git status |
| `exec()` | inherit | inherit | Interactive commands (yay, sudo, git clone) |

```rust
use chezfl::cmd::{cmd, run_cmd};

// Quick one-shot (captured)
let out = run_cmd("which", &["rg"])?;

// Builder with capture
let out = cmd("git")
    .args(&["-C", "/home/user/src/foo"])
    .args(&["status", "--porcelain"])
    .run()?;

// Interactive (sudo prompts forwarded to terminal)
cmd("sudo").args(&["pacman", "-Syu"]).exec()?;

// With timeout and retry
let out = cmd("ping")
    .arg("-c").arg("1").arg("10.0.0.1")
    .timeout(std::time::Duration::from_secs(5))
    .retry(2)
    .run()?;
```

## Built-in tools

chezfl ships with convenience wrappers for common programs in
[`chezfl::tools`](https://docs.rs/chezfl/latest/chezfl/tools/index.html):

### `tools::yay`

```rust
use chezfl::tools::yay;

yay::install(&["ripgrep", "fd"])?;      // yay -S (interactive)
yay::remove(&["firefox"])?;             // yay -R (interactive)
yay::remove_recursive(&["firefox"])?;   // yay -Rs (interactive)
yay::update()?;                         // yay -Syu (interactive)
let installed = yay::is_installed("ripgrep")?;
```

### `tools::git`

```rust
use chezfl::tools::git;

git::clone("https://github.com/user/repo", "/home/user/src/repo")?;
git::pull("/home/user/src/repo")?;
git::fetch("/home/user/src/repo")?;
let out = git::status("/home/user/src/repo")?;
let clean = git::is_clean("/home/user/src/repo")?;
```

## Examples

The `examples/` directory contains ready-to-run config templates:

| Example | API | Focus |
|---------|-----|-------|
| [`examples/desktop.rs`](examples/desktop.rs) | Builder | yay + git tools, multi-target dependency graph |
| [`examples/laptop.rs`](examples/laptop.rs) | Macro | concise config using `target!`/`task!`/`run!` |
| [`examples/cmd_demo.rs`](examples/cmd_demo.rs) | Builder | Cmd API features: run, exec, timeout, retry, env, dir |

```bash
# Run the desktop config (try check first, then apply)
cargo run --example desktop check
cargo run --example desktop plan
cargo run --example desktop apply

# Run the laptop config
cargo run --example laptop check

# Run the Cmd API demo
cargo run --example cmd_demo
```

## CLI Reference

```
Usage: my_config [COMMAND]

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
cargo run -- check

# Check only specific targets (includes transitive deps)
cargo run -- check rg_installed

# Only run "install" labelled tasks
cargo run -- apply --label install

# Skip "system" labelled tasks
cargo run -- apply --exclude-label system

# Manually mark a target as satisfied
cargo run -- check --set docker_installed=true

# Force re-check
cargo run -- check --recheck docker_installed
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

- `src/lib.rs` — library root; `src/bin/my_config.rs` — user's personal config binary
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
