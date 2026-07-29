//! Example: Declare a desktop machine's desired state.
//!
//! Run with: `cargo run --example desktop [check|plan|apply]`
//!
//! This example uses the **builder API** to define targets and tasks,
//! and the built-in `yay` and `git` tools inside task closures.

use chezfl::{
    App, Target, Task,
    cmd::{cmd, run_cmd},
    run_cli,
    tools::{git, yay},
};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    // ── targets ─────────────────────────────────────────────────────

    app.target(Target::new("network")); // aggregate — deps determine satisfaction

    app.target(
        Target::new("rg_installed")
            .check(|| yay::is_installed("ripgrep"))
            .depends_on("network"),
    );

    app.target(
        Target::new("fd_installed")
            .check(|| yay::is_installed("fd"))
            .depends_on("network"),
    );

    app.target(
        Target::new("chezfl_repo")
            .check(|| {
                Ok(cmd("test")
                    .args(&["-d", "/home/user/src/chezfl"])
                    .run()
                    .is_ok())
            })
            .depends_on("network"),
    );

    app.target(
        Target::new("chezfl_built")
            .check(|| {
                Ok(run_cmd("test", &["-f", "/home/user/src/chezfl/target/release/chezfl"])
                    .is_ok())
            })
            .depends_on("chezfl_repo"),
    );

    // ── tasks ───────────────────────────────────────────────────────

    app.task(
        Task::new("install_rg")
            .satisfies("rg_installed")
            .depends_on("network")
            .label("install")
            .run(|| {
                yay::install(&["ripgrep"])?;
                Ok(())
            }),
    );

    app.task(
        Task::new("install_fd")
            .satisfies("fd_installed")
            .depends_on("network")
            .label("install")
            .run(|| {
                yay::install(&["fd"])?;
                Ok(())
            }),
    );

    app.task(
        Task::new("clone_chezfl")
            .satisfies("chezfl_repo")
            .depends_on("network")
            .label("clone")
            .run(|| {
                git::clone(
                    "https://github.com/user/chezfl",
                    "/home/user/src/chezfl",
                )?;
                Ok(())
            }),
    );

    app.task(
        Task::new("build_chezfl")
            .satisfies("chezfl_built")
            .depends_on("chezfl_repo")
            .label("build")
            .run(|| {
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
