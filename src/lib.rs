/// Internal global registry for the macro-based API.
///
/// See [`target!`], [`task!`], and [`run!`] macros.
pub mod __internals;
pub mod app;
pub mod cli;
pub mod cmd;
#[macro_use]
pub mod macros;
pub mod state;
pub mod target;
pub mod task;
pub mod tools;

pub use app::{App, Config};
use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
pub use cmd::{Cmd, Output as CmdOutput, cmd, run_cmd};
pub use target::Target;
pub use task::Task;

const BANNER: &str = r"
  _____    __   __      _____   _____        _____    __      
 /\ __/\  /\_\ /_/\   /\_____\ /\____\     /\_____\  /\_\     
 ) )__\/ ( ( (_) ) ) ( (_____/ \/_ ( (    ( (  ___/ ( ( (     
/ / /     \ \___/ /   \ \__\      \ \_\    \ \ \_    \ \_\    
\ \ \_    / / _ \ \   / /__/_     / / /__  / / /_\   / / /__  
 ) )__/\ ( (_( )_) ) ( (_____\   ( (____( / /____/  ( (_____( 
 \/___\/  \/_/ \_\/   \/_____/    \/____/ \/_/       \/_____/ 
";

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
    pub description: Option<String>,
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
    use std::io::IsTerminal;
    let cli = cli::Cli::parse();

    if !cli.no_banner {
        if std::io::stdout().is_terminal() {
            for line in BANNER
                .trim_start_matches('\n')
                .trim_end_matches('\n')
                .lines()
            {
                println!("\x1b[48;5;183m\x1b[30m{}\x1b[0m", line);
            }
        } else {
            print!("{BANNER}");
        }
    }

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
    // Persist immediately so manual overrides survive even if a later
    // subcommand crashes or doesn't save (e.g. plan).
    if !cli.set.is_empty() || !cli.unset.is_empty() || !cli.recheck.is_empty() {
        let _ = app.save_state();
    }

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

    let show_desc = cli.show_descriptions;

    match &command {
        cli::Command::Check { targets } => {
            let steps = app.run_check(&config, targets);
            print_steps(&steps, false, show_desc);
        }
        cli::Command::Plan { targets } => {
            let steps = app.run_plan(&config, targets);
            print_steps(&steps, true, show_desc);
        }
        cli::Command::Apply { targets } => {
            let steps = app.run_apply(&config, targets);
            print_steps(&steps, false, show_desc);
        }
        cli::Command::Completions { shell } => {
            let bin_name = std::env::args().next()
                .and_then(|p| std::path::Path::new(&p).file_name().map(|s| s.to_os_string()))
                .and_then(|s| s.into_string().ok())
                .unwrap_or_else(|| "chezfl".into());
            let mut cmd = cli::Cli::command();
            generate(*shell, &mut cmd, &bin_name, &mut std::io::stdout());
        }
    }

    Ok(())
}

fn print_steps(steps: &[Step], is_plan: bool, show_descriptions: bool) {
    use std::io::IsTerminal;

    let color = std::io::stdout().is_terminal();

    let green = |s: &str| {
        if color {
            format!("\x1b[32m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };
    let red = |s: &str| {
        if color {
            format!("\x1b[31m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };
    let yellow = |s: &str| {
        if color {
            format!("\x1b[33m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };
    let dim = |s: &str| {
        if color {
            format!("\x1b[2m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };
    let red_strike = |s: &str| {
        if color {
            format!("\x1b[31;9m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };
    let bold = |s: &str| {
        if color {
            format!("\x1b[1m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    };

    for step in steps {
        let icon = match step.sat {
            crate::Satisfaction::Satisfied => green("✓"),
            crate::Satisfaction::Unsatisfied => red("✗"),
        };
        let name = bold(&step.name);
        let detail = if step.detail.is_empty() {
            String::new()
        } else {
            format!("  {}", dim(&format!("({})", step.detail)))
        };
        let desc = match &step.description {
            Some(d) if show_descriptions || step.sat == crate::Satisfaction::Unsatisfied => {
                if step.sat == crate::Satisfaction::Unsatisfied {
                    format!("  {}", red_strike(d))
                } else {
                    format!("  {}", dim(d))
                }
            }
            _ => String::new(),
        };
        let prefix = if is_plan {
            format!("{} ", yellow("(P)"))
        } else {
            String::new()
        };
        println!("{}{} {}{}{}", prefix, icon, name, desc, detail);
    }
}
