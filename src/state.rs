use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Persisted state for a single target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetState {
    pub satisfied: Option<bool>,
    pub manually_set: bool,
    pub checked_at: Option<String>,
}

/// TOML-serialised persistence of target satisfaction state.
///
/// Stored at `~/.local/state/chezfl/state.toml` by default. Supports
/// manual overrides (via `--set`/`--unset`) so users can mark targets
/// as satisfied without running their check function.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    targets: HashMap<String, TargetState>,
}

impl State {
    /// Load state from the default path (`~/.local/state/chezfl/state.toml`).
    ///
    /// Returns a default (empty) state if the file does not exist or
    /// cannot be read.
    pub fn load() -> Self {
        Self::load_from(None)
    }

    /// Load state from an optional custom path.
    ///
    /// `None` falls back to [`default_path`].
    pub fn load_from(path: Option<PathBuf>) -> Self {
        let path = path.unwrap_or_else(default_path);
        if !path.exists() {
            return State::default();
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return State::default(),
        };
        toml::from_str(&content).unwrap_or_default()
    }

    /// Persist state to the given file path.
    pub fn save_to(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Look up a target's persisted state.
    pub fn get(&self, name: &str) -> Option<&TargetState> {
        self.targets.get(name)
    }

    /// Manually set a target's satisfaction (bypasses check).
    ///
    /// This is used by the `--set` CLI flag.
    pub fn set(&mut self, name: &str, satisfied: bool) {
        self.targets.insert(
            name.to_string(),
            TargetState {
                satisfied: Some(satisfied),
                manually_set: true,
                checked_at: Some(chrono_now()),
            },
        );
    }

    /// Remove a target's persisted state (next check will re-run).
    ///
    /// This is used by the `--unset` and `--recheck` CLI flags.
    pub fn unset(&mut self, name: &str) {
        self.targets.remove(name);
    }

    /// Record the result of a check or re-check.
    ///
    /// Preserves `manually_set` if the target was previously manually
    /// overridden, so that `--set` persists across `apply`/`check` runs.
    pub fn set_check_result(&mut self, name: &str, satisfied: bool) {
        let manually_set = self
            .targets
            .get(name)
            .map(|ts| ts.manually_set)
            .unwrap_or(false);
        self.targets.insert(
            name.to_string(),
            TargetState {
                satisfied: Some(satisfied),
                manually_set,
                checked_at: Some(chrono_now()),
            },
        );
    }
}

/// The default path for the state file:
/// `$HOME/.local/state/chezfl/state.toml`
pub fn default_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".local/state/chezfl/state.toml")
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", dur.as_secs())
}
