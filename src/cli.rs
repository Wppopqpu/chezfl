use clap::{Parser, Subcommand};

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
}

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
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}
