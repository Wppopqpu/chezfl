use chezfl::{App, run_cli};
// Available types and tools (uncomment as needed):
// use chezfl::{Target, Task};
// use chezfl::tools::{git, yay};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();

    // ── targets ─────────────────────────────────────────────────

    // Aggregate target (satisfaction from deps)
    // app.target(Target::new("name"));
    // app.target(Target::new("name").description("..."));
    //
    // Leaf target with check function
    // app.target(
    //     Target::new("name")
    //         .description("...")
    //         .check(|| Ok(true))
    //         .depends_on("dep"),
    // );

    // ── tasks ───────────────────────────────────────────────────

    // app.task(
    //     Task::new("name")
    //         .description("...")
    //         .satisfies("target_name")
    //         .depends_on("dep")
    //         .label("install")
    //         .run(|| {
    //             Ok(())
    //         }),
    // );

    // ── run ─────────────────────────────────────────────────────

    run_cli(&mut app)
}
