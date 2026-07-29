# chezfl

A personal system state manager inspired by Nix, written in Rust. Users declare desired system state targets and the tasks that achieve them by writing Rust code using chezfl's library API — the declaration is Rust source code.

chezfl fills the gap that tools like `stow` (dotfile symlinks) cannot cover: package installation, repository cloning and building, systemd unit management, and other stateful targets.

## Language

**Target**:
A declaration of a **concrete** desired state. Each target has a unique **name** (string identifier). There are two kinds:

- **Leaf target**: has a **check** function (`fn() -> anyhow::Result<bool>`) that probes the real system. `Ok(true)` = satisfied, `Ok(false)` = unsatisfied, `Err` = check itself failed.
- **Aggregate target**: no check function. Its satisfaction is derived entirely from its dependencies — it is satisfied when all its dependency targets are satisfied.

A target may declare dependencies on other targets. The dependency graph must be acyclic. A target can only be satisfied by **exactly one** task.

Targets are **concrete** — one target = one specific desired state. Users create multiple targets programmatically (e.g., looping over repos). There is no template abstraction.

*Examples*: "package ripgrep is installed", "repo ~/src/chezfl is cloned and built", "service nginx is running".
*Avoid*: Config entry, setting, value

**Cmd**:
A command runner building on `std::process::Command`. Two execution modes:
- **`run()`** — captures stdout/stderr into a `Output { status, stdout, stderr }` struct, stdin is null.
- **`exec()`** — inherits stdin/stdout/stderr from the parent (fully interactive), returns `Output` with only `status` populated.

Both return `anyhow::Result<Output>`. Supports `.timeout(Duration)` and `.retry(n)`.
Available as a builder (`cmd("git").arg("clone").exec()`) or a one-shot function (`run_cmd("git", &["clone", url])`).

**Tool**:
A convenience wrapper function that uses `Cmd` internally. Organized by program name under `chezfl::tools::*` (e.g., `tools::git::clone`, `tools::yay::install`).
Tools are designed to be called inside a Task's `run` closure — they are helpers for the *how*, not the *what*.
*Avoid*: Plugin, extension

**Task**:
An actionable unit that **satisfies one or more targets** (1-to-many). A task declares which targets it satisfies and which other targets must be satisfied before it can run. A task does NOT depend on other tasks — only on targets.

Tasks have **labels** used for filtering and disabling. A disabled task is completely ignored — its targets are skipped. A task should be idempotent, designed to be run repeatedly. Its run function signature is `fn() -> anyhow::Result<()>`.

Task execution is **serial** (one at a time) — interactive programs like `yay` may prompt the user during execution. chezfl forwards stdin to task processes and captures/displays stdout and stderr along with the command being run.

There is no rollback or uninstall support. Teardown is the user's responsibility.

*Avoid*: Job, script

**State file**:
chezfl persists target satisfaction state to a TOML file (default: `~/.local/state/chezfl/state.toml`). Each target stores whether it was last seen as satisfied, when it was checked, and whether it was manually overridden. On `check`, chezfl reads cached state first; on `apply`, it re-checks only targets whose upstream dependencies have changed or that are explicitly rechecked.

**Manual override**:
Users can set or unset a target's satisfaction via CLI: `--set <target>=satisfied`, `--unset <target>`, `--recheck <target>`. Manually set targets skip their check function until explicitly rechecked.

**Configuration-as-Code**:
Users declare targets and tasks by calling chezfl's Rust API directly from Rust source files. There is no separate config language, no YAML/TOML, and no DSL parser.

chezfl supports two API styles:
- **App builder**: `chezfl::App::new().target(...).task(...)` — explicit, composable
- **Global macros**: `target!(...); task!(...); chezfl::cli::run!()` — concise, convenient

Both produce the same runtime model. Users compile their config into a binary and run subcommands:

- `./my-config [apply]` — default: check all targets, run tasks for unsatisfied ones, re-check after each task
- `./my-config check [target...]` — check targets (default all), report satisfaction state
- `./my-config plan` — dry-run: simulate task execution (assume satisfied after) without side effects, output in text-tree format
- `./my-config check --label foo --exclude-label bar` — filter targets by task labels
*Avoid*: Config file, config language, DSL
