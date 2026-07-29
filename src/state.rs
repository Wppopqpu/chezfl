use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetState {
    pub satisfied: Option<bool>,
    pub manually_set: bool,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    targets: HashMap<String, TargetState>,
}

impl State {
    pub fn load() -> Self {
        Self::load_from(None)
    }

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

    pub fn save_to(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&TargetState> {
        self.targets.get(name)
    }

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

    pub fn unset(&mut self, name: &str) {
        self.targets.remove(name);
    }

    pub fn set_check_result(&mut self, name: &str, satisfied: bool) {
        self.targets.insert(
            name.to_string(),
            TargetState {
                satisfied: Some(satisfied),
                manually_set: false,
                checked_at: Some(chrono_now()),
            },
        );
    }
}

fn default_path() -> PathBuf {
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
