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
- **At most once per apply** — a task is never run twice in a single run;
  idempotency is not required.
- **Best-effort** — if a dependency can't be satisfied, dependent targets
  are skipped but the rest continues.

## Quickstart

### Clone the repo and create a config branch

```bash
git clone https://github.com/wppopqpu/chezfl
cd chezfl
git checkout -b my-config        # your personal config branch
```

### Write your config (`src/bin/chezfl.rs`)

A ready-to-use template is provided at [`src/bin/chezfl.rs`](src/bin/chezfl.rs).
Uncomment and fill in your own targets and tasks:

```rust
use chezfl::{App, Target, Task, run_cli};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    // Aggregate target — no check, satisfied when all deps are satisfied
    app.target(Target::new("network"));

    // Leaf target — check guarded by check dependency
    app.target(
        Target::new("rg_installed")
            .check_dep("network")
            .check(|| chezfl::tools::yay::is_installed("ripgrep")),
    );

    // Aggregate — satisfies when both network and rg are ready
    app.target(
        Target::new("rg_ready")
            .depends_on("network")
            .depends_on("rg_installed"),
    );

    // Task satisfies an aggregate target (runs when deps allow, re-checks after)
    app.task(
        Task::new("install_rg")
            .satisfies("rg_ready")
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
        check_dep: [network],
    );

    target!("rg_ready",
        description: "rg is installed and ready",
        depends_on: [network, rg_installed],
    );

    task!("install_rg",
        description: "Install ripgrep via yay",
        satisfies: [rg_ready],
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

See [`examples/laptop.rs`](examples/laptop.rs) and [`examples/desktop.rs`](examples/desktop.rs) for full macro/builder examples.

### Build and run

```bash
# Default binary is chezfl (set in Cargo.toml)
cargo run -- check          # check all targets
cargo run -- plan           # plan (dry-run)
cargo run                   # apply (default subcommand)

# Or build once and use the binary directly
cargo build --release
./target/release/chezfl check
```

## Workflow: keep your config in sync with chezfl

chezfl and your config live in the same repository. A two-branch workflow
keeps them separate:

```text
main          — chezfl library source (receives upstream updates)
my-config     — your branch: src/bin/chezfl.rs + chezfl source
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

This approach works because `src/bin/chezfl.rs` is gitignored from the
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
[`chezfl::tools`](https://docs.rs/chezfl/latest/chezfl/tools/index.html).

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
let is_repo = git::is_git_repo("/home/user/src/repo")?;
let fresh = git::is_up_to_date_with_git("/home/user/src/repo/target", "/home/user/src/repo")?;
git::switch("/home/user/src/repo", "dev")?;              // git switch dev
git::switch_create("/home/user/src/repo", "my-config")?; // git switch -c my-config
```

`git::clone` takes git-default options; use the [`CloneOptions`](src/tools/git.rs)
builder to control branches and submodules:

```rust
use chezfl::tools::git::CloneOptions;

// clone every branch and all submodules, checking out `main`
CloneOptions::new()
    .all_branches()                 // --no-single-branch
    .submodules()                   // --recurse-submodules
    .branch("main")                 // --branch main (default checkout)
    .clone("https://github.com/user/repo", "/home/user/src/repo")?;

// fetch every branch explicitly (no default branch override)
CloneOptions::new()
    .all_branches()                 // --no-single-branch
    .clone("https://github.com/user/repo", "/home/user/src/repo")?;

// fetch only one branch and check it out
CloneOptions::new()
    .single_branch()                // --single-branch
    .branch("main")                 // --branch main
    .clone("https://github.com/user/repo", "/home/user/src/repo")?;
```

### `tools::cargo`

```rust
use chezfl::tools::cargo;

cargo::install("bat")?;                          // cargo install bat
cargo::install_path("/home/user/src/tool")?;     // cargo install --path ...
cargo::install_root("rg", "/home/user/.local")?; // cargo install --root ...
cargo::install_force("bat")?;                    // cargo install --force bat
let installed = cargo::is_installed("bat")?;
```

### `tools::stow`

```rust
use chezfl::tools::stow;

stow::stow("~/dotfiles", "~", "bash")?;         // stow -S
stow::unstow("~/dotfiles", "~", "bash")?;       // stow -D
stow::restow("~/dotfiles", "~", "bash")?;       // stow -R
stow::stow_everything("~/dotfiles", "~")?;      // stow every package dir
let stowed = stow::is_stowed("~/dotfiles", "~", "bash")?;
let all = stow::is_everything_stowed("~/dotfiles", "~")?;
```

### `tools::systemd`

```rust
use chezfl::tools::systemd::Systemctl;

Systemctl::new().enable("foo.service")?;          // sudo systemctl enable
Systemctl::new().now().enable("foo.service")?;    // sudo systemctl enable --now
Systemctl::new().user().start("foo.service")?;    // systemctl --user start
Systemctl::new().daemon_reload()?;                // sudo systemctl daemon-reload

let running = Systemctl::new().is_unit_running("foo.service")?;
```

System units run via `sudo` (never polkit). `--user` units run directly
without sudo. Read-only checks (`is_unit_running`) never prompt. Free
helpers are also provided: `systemd::enable`, `systemd::enable_now`,
`systemd::daemon_reload`, `systemd::start`, `systemd::stop`,
`systemd::restart`, `systemd::is_unit_running`.

### `tools::mime`

```rust
use chezfl::tools::mime;

let default = mime::query_default("text/plain")?;
let is_nvim = mime::is_default("text/plain", "nvim.desktop")?;
mime::set_default("text/plain", "nvim.desktop")?;
```

### `tools::fs`

```rust
use chezfl::tools::fs;

fs::is_file("/etc/passwd")?;
fs::is_dir("/home/user")?;
fs::is_symlink("/usr/local/bin/rg")?;
fs::is_runnable("/usr/bin/yay")?;
fs::exists("/tmp/x")?;
fs::read_to_string("/tmp/x")?;
fs::write("/tmp/x", "content")?;
fs::copy("/tmp/x", "/tmp/y")?;
fs::remove("/tmp/x")?;
fs::remove_all("/tmp/dir")?;
fs::create_dir("/tmp/a/b")?;
fs::symlink("/tmp/x", "/tmp/link")?;
let mtime = fs::mtime("/tmp/x")?;
let fresh = fs::up_to_date("/tmp/x", &["/tmp/src"])?;
```

Predicates return `false` (never error) for missing paths; operations
attach the offending path to their error messages.

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
Usage: chezfl [COMMAND]

Commands:
  check   Check target satisfaction (no side effects)
  plan    Plan — simulate apply without running tasks
  apply   Apply — converge toward desired state

Flags (every command):
  --label <LABEL>         Only consider tasks with this label (repeatable)
  --exclude-label <LABEL> Exclude tasks with this label (repeatable)
  --set <NAME=bool>       Manually set target state (bypasses check, repeatable)
  --unset <NAME>          Remove stored state for a target (repeatable)
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

# Clear stored state (next check runs fresh)
cargo run -- check --unset docker_installed
```

## Domain Model

See [CONTEXT.md](CONTEXT.md) for the full glossary and design rationale.

### Targets

A **Target** is a concrete desired state. Three kinds:

- **Leaf target** — has a `check` closure that probes the real system
  (e.g. "is ripgrep installed?"). Optionally declares **check dependencies**
  (`check_dep`) — the check only runs when all check deps are satisfied.
  If a check dep is unsatisfied, or if the check returns `false`, the leaf
  is demoted to stub (cannot be satisfied by a task). A leaf's status is
  decided **solely** by its own check and check deps — never by whether its
  satisfying task is able to run.
- **Aggregate target** — no check; satisfied when all its `depends_on`
  dependencies are satisfied. Useful for grouping. Tasks satisfy aggregates.
- **Stub target** — neither check nor deps; always unsatisfied unless
  manually set via `--set`. A task declaring `satisfies` for a stub target
  is silently skipped.

Each target is satisfied by **exactly one** task. Targets form an
acyclic dependency DAG. The dimensions of `depends_on` and `check_dep`
are orthogonal — `depends_on` controls topological ordering and aggregate
derivation; `check_dep` guards whether a leaf's check runs.

### Tasks

A **Task** is an actionable unit that satisfies 1+ targets.

- Has a `run` closure (at most once per apply, serial, stdin-forwarded)
- Declares labels for filtering
- Depends on *targets* (not other tasks) — this keeps the dependency model
  simple and avoids redundant execution. A task's `depends_on` gates **only
  when it runs**: an unsatisfied dependency leaves the task unexecuted but
  never changes the status of the targets it satisfies, and never cascades
  to block downstream targets.
- No rollback

## Conventions

- `src/lib.rs` — library root; `src/bin/chezfl.rs` — user's personal config binary
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
