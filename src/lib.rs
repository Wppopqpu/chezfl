/// Internal global registry for the macro-based API.
///
/// See [`target!`], [`task!`], and [`run!`] macros.
pub mod __internals;
pub mod app;
pub mod cli;
#[macro_use]
pub mod macros;
pub mod state;
pub mod target;
pub mod task;

pub use app::{App, Config};
use clap::Parser;
pub use target::Target;
pub use task::Task;

/// Whether a target was satisfied after a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Satisfaction {
    Satisfied,
    Unsatisfied,
}

/// One entry in the output of a check/apply/plan run.
///
/// Each step corresponds to one target (or one re-check after a task).
#[derive(Debug, Clone)]
pub struct Step {
    pub name: String,
    pub sat: Satisfaction,
    pub detail: String,
}

/// Build an [`App`] via the global registry, parse CLI args, and run.
///
/// This is the entry point for the **global-macro** API style:
///
/// ```ignore
/// target!("net");
/// target!("rg", check: || which("rg").is_ok(), depends_on: [net]);
/// task!("install_rg", satisfies: [rg], run: || install("rg"));
/// run!();  // parses argv, calls run_cli
/// ```
///
/// For the **builder** style use [`App`] methods directly and call this with
/// the built app:
///
/// ```ignore
/// let mut app = App::new();
/// app.target(Target::new("rg").check(|| which("rg").is_ok()));
/// app.task(Task::new("install_rg").satisfies("rg").run(|| ...));
/// run_cli(&mut app)
/// ```
pub fn run_cli(app: &mut App) -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    // Handle --set / --unset / --recheck
    for s in &cli.set {
        if let Some((name, _)) = s.split_once('=') {
            app.state_mut().set(name, true);
        } else {
            app.state_mut().set(s, true);
        }
    }
    for s in &cli.unset {
        app.state_mut().unset(s);
    }
    for s in &cli.recheck {
        app.state_mut().unset(s);
    }
    // State is saved during apply, or we can provide a --save flag later

    let config = Config {
        label_filter: if cli.label.is_empty() {
            None
        } else {
            Some(cli.label)
        },
        exclude_labels: cli.exclude_label,
    };

    app.validate()?;

    let command = cli.command.unwrap_or(cli::Command::Apply {
        targets: Vec::new(),
    });

    match &command {
        cli::Command::Check { targets } => {
            let steps = app.run_check(&config, targets);
            print_steps(&steps, false);
        }
        cli::Command::Plan { targets } => {
            let steps = app.run_plan(&config, targets);
            print_steps(&steps, true);
        }
        cli::Command::Apply { targets } => {
            let steps = app.run_apply(&config, targets);
            print_steps(&steps, false);
        }
    }

    Ok(())
}

fn print_steps(steps: &[Step], is_plan: bool) {
    let prefix = if is_plan { "(P) " } else { "" };
    for step in steps {
        let icon = match step.sat {
            crate::Satisfaction::Satisfied => "✓",
            crate::Satisfaction::Unsatisfied => "✗",
        };
        let detail = if step.detail.is_empty() {
            String::new()
        } else {
            format!("  ({})", step.detail)
        };
        println!("{}{} {}{}", prefix, icon, step.name, detail);
    }
}
