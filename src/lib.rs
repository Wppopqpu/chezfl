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
pub mod term;
pub mod tools;

pub use app::{App, Config};
use clap::CommandFactory;
use clap::Parser;
use clap_complete::CompleteEnv;
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

    cli::populate_completion_lists(app);

    CompleteEnv::with_factory(cli::Cli::command).complete();

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

    // Handle --set / --unset
    for s in &cli.set {
        let (name, value) = parse_set_flag(s)?;
        app.state_mut().set(&name, value);
    }
    for s in &cli.unset {
        app.state_mut().unset(s);
    }
    // Persist immediately so manual overrides survive even if a later
    // subcommand crashes or doesn't save (e.g. plan).
    if !cli.set.is_empty() || !cli.unset.is_empty() {
        let _ = app.save_state();
    }

    let config = Config {
        label_filter: if cli.label.is_empty() {
            None
        } else {
            Some(cli.label)
        },
        exclude_labels: cli.exclude_label,
        show_descriptions: cli.show_descriptions,
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
    }

    Ok(())
}

/// Parse a `--set` flag value: `NAME` pins `true`, `NAME=bool` pins the
/// given value.
fn parse_set_flag(s: &str) -> anyhow::Result<(String, bool)> {
    match s.split_once('=') {
        Some((name, value)) => {
            let value = value.parse::<bool>().map_err(|_| {
                anyhow::anyhow!(
                    "invalid --set value '{value}' for '{name}', expected true or false"
                )
            })?;
            Ok((name.to_string(), value))
        }
        None => Ok((s.to_string(), true)),
    }
}

fn print_steps(steps: &[Step], is_plan: bool, show_descriptions: bool) {
    for step in steps {
        let icon = match step.sat {
            crate::Satisfaction::Satisfied => term::green("✓"),
            crate::Satisfaction::Unsatisfied => term::red("✗"),
        };
        let name = term::bold(&step.name);
        let detail = if step.detail.is_empty() {
            String::new()
        } else {
            format!("  {}", term::dim(&format!("({})", step.detail)))
        };
        let desc = match &step.description {
            Some(d) if show_descriptions || step.sat == crate::Satisfaction::Unsatisfied => {
                if step.sat == crate::Satisfaction::Unsatisfied {
                    format!("  {}", term::red_strike(d))
                } else {
                    format!("  {}", term::dim(d))
                }
            }
            _ => String::new(),
        };
        let prefix = if is_plan {
            format!("{} ", term::yellow("(P)"))
        } else {
            String::new()
        };
        println!("{}{} {}{}{}", prefix, icon, name, desc, detail);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_set_flag;

    #[test]
    fn parse_set_plain_name_defaults_true() {
        assert_eq!(parse_set_flag("foo").unwrap(), ("foo".to_string(), true));
    }

    #[test]
    fn parse_set_honors_bool_value() {
        assert_eq!(
            parse_set_flag("foo=true").unwrap(),
            ("foo".to_string(), true)
        );
        assert_eq!(
            parse_set_flag("foo=false").unwrap(),
            ("foo".to_string(), false)
        );
    }

    #[test]
    fn parse_set_rejects_invalid_value() {
        let err = parse_set_flag("foo=yes").unwrap_err();
        assert!(err.to_string().contains("expected true or false"));
    }
}
