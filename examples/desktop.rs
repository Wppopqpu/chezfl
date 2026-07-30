//! Example: Declare a desktop machine's desired state.
//!
//! Run with: `cargo run --example desktop [check|plan|apply]`
//!
//! This example uses the **builder API** to define targets and tasks,
//! and the built-in `yay` and `git` tools inside task closures.
//!
//! Realistic `check_dep` patterns:
//! - A program must be installed before its config can be checked.
//! - A source directory must exist before a binary can be built.

use chezfl::{
    App, Target, Task,
    cmd::{cmd, run_cmd},
    run_cli,
    tools::{git, yay},
};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    // ── targets ─────────────────────────────────────────────────────

    // ── ripgrep ecosystem ───────────────────────────────────────────

    // Leaf: no check_dep — fundamental check, nothing to guard it
    app.target(
        Target::new("rg_binary")
            .description("ripgrep (rg) binary exists")
            .check(|| yay::is_installed("ripgrep")),
    );

    // Leaf: guarded by rg_binary — can't check config if rg isn't installed
    app.target(
        Target::new("rg_conf")
            .description("rg.conf is present")
            .check_dep("rg_binary")
            .check(|| Ok(cmd("test").args(&["-f", "/etc/rg.conf"]).run().is_ok())),
    );

    // Aggregate: task satisfies this, which re-checks both leaf deps
    app.target(
        Target::new("rg_ready")
            .description("ripgrep is installed and configured")
            .depends_on("rg_binary")
            .depends_on("rg_conf"),
    );

    // ── fd ecosystem ────────────────────────────────────────────────

    app.target(
        Target::new("fd_binary")
            .description("fd binary exists")
            .check(|| yay::is_installed("fd")),
    );

    app.target(
        Target::new("fd_conf")
            .description("fd.conf is present")
            .check_dep("fd_binary")
            .check(|| Ok(cmd("test").args(&["-f", "/etc/fd.conf"]).run().is_ok())),
    );

    app.target(
        Target::new("fd_ready")
            .description("fd is installed and configured")
            .depends_on("fd_binary")
            .depends_on("fd_conf"),
    );

    // ── chezfl build ────────────────────────────────────────────────

    app.target(
        Target::new("chezfl_dir")
            .description("chezfl source directory exists")
            .check(|| {
                Ok(cmd("test")
                    .args(&["-d", "/home/user/src/chezfl"])
                    .run()
                    .is_ok())
            }),
    );

    app.target(
        Target::new("chezfl_bin")
            .description("chezfl release binary exists")
            .check_dep("chezfl_dir")
            .check(|| {
                Ok(run_cmd(
                    "test",
                    &["-f", "/home/user/src/chezfl/target/release/chezfl"],
                )
                .is_ok())
            }),
    );

    // Aggregate: single task clones + builds, then both leaf deps re-check
    app.target(
        Target::new("chezfl_built")
            .description("chezfl is cloned and built")
            .depends_on("chezfl_dir")
            .depends_on("chezfl_bin"),
    );

    // ── tasks ───────────────────────────────────────────────────────

    app.task(
        Task::new("install_rg")
            .description("Install ripgrep via yay")
            .satisfies("rg_ready")
            .label("install")
            .run(|| {
                yay::install(&["ripgrep"])?;
                Ok(())
            }),
    );

    app.task(
        Task::new("install_fd")
            .description("Install fd via yay")
            .satisfies("fd_ready")
            .label("install")
            .run(|| {
                yay::install(&["fd"])?;
                Ok(())
            }),
    );

    app.task(
        Task::new("setup_chezfl")
            .description("Clone and build chezfl")
            .satisfies("chezfl_built")
            .run(|| {
                git::clone("https://github.com/user/chezfl", "/home/user/src/chezfl")?;
                cmd("cargo")
                    .args(&["build", "--release"])
                    .dir("/home/user/src/chezfl")
                    .exec()?;
                Ok(())
            }),
    );

    // ── run ─────────────────────────────────────────────────────────

    run_cli(&mut app)
}
