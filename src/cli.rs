use std::ffi::OsStr;

use clap::{Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

use crate::app::App;

pub(crate) static TARGET_NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
pub(crate) static LABEL_NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

pub(crate) fn populate_completion_lists(app: &App) {
    let targets: Vec<String> = app.all_targets().map(|t| t.name.clone()).collect();
    let _ = TARGET_NAMES.set(targets);
    let mut labels: Vec<String> = app.all_tasks().flat_map(|t| t.labels.clone()).collect();
    labels.sort();
    labels.dedup();
    let _ = LABEL_NAMES.set(labels);
}

fn complete_targets(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(names) = TARGET_NAMES.get() else {
        return vec![];
    };
    let current = current.to_string_lossy();
    names
        .iter()
        .filter(|n| n.starts_with(current.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

fn complete_labels(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(names) = LABEL_NAMES.get() else {
        return vec![];
    };
    let current = current.to_string_lossy();
    names
        .iter()
        .filter(|n| n.starts_with(current.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

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
    #[arg(long, global = true, add = ArgValueCompleter::new(complete_labels))]
    pub label: Vec<String>,

    /// Exclude tasks with label
    #[arg(long, global = true, add = ArgValueCompleter::new(complete_labels))]
    pub exclude_label: Vec<String>,

    /// Manually set target satisfaction
    #[arg(long, global = true, value_name = "TARGET", add = ArgValueCompleter::new(complete_targets))]
    pub set: Vec<String>,

    /// Manually unset target satisfaction
    #[arg(long, global = true, value_name = "TARGET", add = ArgValueCompleter::new(complete_targets))]
    pub unset: Vec<String>,

    /// Re-check a target (clear cached state)
    #[arg(long, global = true, value_name = "TARGET", add = ArgValueCompleter::new(complete_targets))]
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
        #[arg(add = ArgValueCompleter::new(complete_targets))]
        targets: Vec<String>,
    },
    /// Plan what would change (dry-run)
    Plan {
        /// Targets to plan for (default: all)
        #[arg(add = ArgValueCompleter::new(complete_targets))]
        targets: Vec<String>,
    },
    /// Apply: satisfy targets by running tasks
    Apply {
        /// Targets to apply (default: all)
        #[arg(add = ArgValueCompleter::new(complete_targets))]
        targets: Vec<String>,
    },
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}
