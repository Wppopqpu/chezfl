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
