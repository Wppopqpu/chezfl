use std::path::Path;

use anyhow::Context;

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

/// Check whether `dir` is inside a git work tree.
///
/// Uses `git rev-parse --is-inside-work-tree`. Non-interactive. Suitable
/// for use in a target's `check` function.
pub fn is_git_repo(dir: impl AsRef<Path>) -> anyhow::Result<bool> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    let out = cmd("git")
        .args(&["-C", &dir, "rev-parse", "--is-inside-work-tree"])
        .run()?;
    Ok(out.stdout.trim() == "true")
}

/// Check whether `target` is as new as the latest commit on the current
/// branch of the repository at `dir`.
///
/// Uses modification times: the target is considered up to date when it
/// exists and its mtime is at least as recent as the latest commit
/// timestamp (`git log -1 --format=%ct`).
///
/// Suitable for use in a target's `check` function. Returns `false` when
/// the target does not exist.
pub fn is_up_to_date_with_git(
    target: impl AsRef<Path>,
    dir: impl AsRef<Path>,
) -> anyhow::Result<bool> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    let target_mtime = match crate::tools::fs::mtime(&target)? {
        Some(t) => t,
        None => return Ok(false),
    };

    let out = cmd("git")
        .args(&["-C", &dir, "log", "-1", "--format=%ct"])
        .run()?;
    let latest_commit = out
        .stdout
        .trim()
        .parse::<u64>()
        .with_context(|| format!("unexpected git log output: {:?}", out.stdout))?;

    Ok(latest_commit <= target_mtime)
}
