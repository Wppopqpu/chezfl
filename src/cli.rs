use clap::{Parser, Subcommand};

/// CLI entry point: parse command-line arguments and dispatch.
///
/// Every subcommand supports execution filters
/// ([`--label`](Cli::label)/[`--exclude-label`](Cli::exclude_label))
/// and state overrides
/// ([`--set`](Cli::set)/[`--unset`](Cli::unset)/[`--recheck`](Cli::recheck)).
#[derive(Parser)]
#[command(name = "chezfl", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Filter by task label
    #[arg(long, global = true)]
    pub label: Vec<String>,

    /// Exclude tasks with label
    #[arg(long, global = true)]
    pub exclude_label: Vec<String>,

    /// Manually set target satisfaction
    #[arg(long, global = true, value_name = "TARGET")]
    pub set: Vec<String>,

    /// Manually unset target satisfaction
    #[arg(long, global = true, value_name = "TARGET")]
    pub unset: Vec<String>,

    /// Re-check a target (clear cached state)
    #[arg(long, global = true, value_name = "TARGET")]
    pub recheck: Vec<String>,

    /// Show target descriptions (always shown for unsatisfied targets)
    #[arg(long, global = true, default_value_t = false)]
    pub show_descriptions: bool,

    /// Suppress the startup banner
    #[arg(long, global = true, default_value_t = false)]
    pub no_banner: bool,
}

/// Available subcommands.
#[derive(Subcommand)]
pub enum Command {
    /// Check target satisfaction
    Check {
        /// Targets to check (default: all)
        targets: Vec<String>,
    },
    /// Plan what would change (dry-run)
    Plan {
        /// Targets to plan for (default: all)
        targets: Vec<String>,
    },
    /// Apply: satisfy targets by running tasks
    Apply {
        /// Targets to apply (default: all)
        targets: Vec<String>,
    },
    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}
