use std::path::Path;

use crate::cmd::{Output, cmd};

/// Clone a repository into `dir`.
///
/// Interactive — may prompt for credentials.
pub fn clone(url: &str, dir: impl AsRef<Path>) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    cmd("git").args(&["clone", url, &dir]).exec()
}

/// Pull latest changes in `dir`.
pub fn pull(dir: impl AsRef<Path>) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    cmd("git").args(&["-C", &dir, "pull", "--ff-only"]).exec()
}

/// Fetch from all remotes in `dir`.
pub fn fetch(dir: impl AsRef<Path>) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    cmd("git")
        .args(&["-C", &dir, "fetch", "--all", "--prune"])
        .run()
}

/// Show working-tree status (porcelain format).
pub fn status(dir: impl AsRef<Path>) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    cmd("git")
        .args(&["-C", &dir, "status", "--porcelain"])
        .run()
}

/// Check whether the working tree is clean (no modified/untracked files).
pub fn is_clean(dir: impl AsRef<Path>) -> anyhow::Result<bool> {
    let out = status(dir)?;
    Ok(out.stdout.trim().is_empty())
}
