use chezfl::{App, run_cli};

fn main() -> anyhow::Result<()> {
    let mut app = App::new();
    run_cli(&mut app)
}
