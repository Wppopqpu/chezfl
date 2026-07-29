# chezfl

A personal system state manager inspired by Nix, written in Rust. Users declare desired state (packages, repos, services) as **targets** and define **tasks** that satisfy them by writing Rust code using chezfl's library API.

## Stack

- **Language:** Rust (single crate, no workspace)
- **Build/test/lint:** `cargo build`, `cargo test`, `cargo fmt`, `cargo clippy`
- **No codegen, no build script, no unsafe** unless justified

## Commands

```bash
cargo build               # debug build
cargo build --release     # release build
cargo test                # all tests
cargo test <name>         # single test
cargo fmt                 # format (run before push)
cargo clippy -- -D warnings  # lint (run after fmt)
```

## Conventions

- `src/lib.rs` — library root; `src/bin/my_config.rs` — user's personal config binary (default-run)
- Tests live in `tests/` (integration) and inline `#[cfg(test)] mod tests` (unit)
- Use `anyhow` for error handling; `thiserror` for library errors
- Public API goes through lib; main only parses CLI args and calls lib

## Domain model (see CONTEXT.md for full glossary)

- **Target** — a concrete desired state with a `check` function. Two kinds: leaf (has check) and aggregate (satisfaction from deps). Each target is satisfied by exactly one task. Targets form an acyclic dependency DAG.
- **Task** — satisfies 1+ targets, depends on targets (not tasks), has labels for filtering. Serial execution, idempotent, stdin-forwarded. No rollback.
- **State** — persisted in TOML (`~/.local/state/chezfl/state.toml`). Supports manual override via `--set`/`--unset`/`--recheck`. Check caching: leaf targets skip check when previously satisfied + deps unchanged.
- **Description** — optional human-readable string on targets and tasks via `.description("text")`. Shown always for unsatisfied targets; opt-in via `--show-descriptions` flag.
- **CLI** — `./my-config [apply]`, `./my-config check [target...]`, `./my-config plan`. Supports `--label`, `--exclude-label`, `--show-descriptions`, `--no-banner`.
- **API** — supports both App builder and global macros.
- **ADR-0001** — Configuration as Rust Code, not a DSL.
