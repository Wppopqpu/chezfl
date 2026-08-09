use std::path::Path;

use anyhow::Context;

use crate::cmd::{Output, cmd};

/// Builder for `git clone` invocations.
///
/// Controls which branches are fetched and whether submodules are cloned.
///
/// # Example
///
/// ```no_run
/// use chezfl::tools::git::CloneOptions;
/// // clone every branch and all submodules, checking out `main`
/// CloneOptions::new()
///     .all_branches()
///     .submodules()
///     .branch("main")
///     .clone("https://github.com/user/repo", "/home/user/src/repo")?;
/// # anyhow::Ok(())
/// ```
#[derive(Clone, Default)]
pub struct CloneOptions {
    branch: Option<String>,
    branches: Branches,
    submodules: bool,
}

#[derive(Clone, Copy, Default)]
enum Branches {
    /// Git default: fetch all branches (no `--single-branch` flag).
    #[default]
    All,
    /// Fetch only the requested branch (`--single-branch`).
    Single,
    /// Fetch all branches explicitly (`--no-single-branch`).
    NoSingle,
}

impl CloneOptions {
    /// Create a builder with git-default clone behavior.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check out `branch` instead of the remote default HEAD
    /// (`--branch <branch>`). Independent of the fetch scope chosen by
    /// [`single_branch`](Self::single_branch)/[`all_branches`](Self::all_branches).
    pub fn branch(mut self, branch: &str) -> Self {
        self.branch = Some(branch.to_string());
        self
    }

    /// Fetch only the requested branch (`--single-branch`).
    pub fn single_branch(mut self) -> Self {
        self.branches = Branches::Single;
        self
    }

    /// Fetch all branches explicitly (`--no-single-branch`).
    pub fn all_branches(mut self) -> Self {
        self.branches = Branches::NoSingle;
        self
    }

    /// Initialize and clone all submodules (`--recurse-submodules`).
    pub fn submodules(mut self) -> Self {
        self.submodules = true;
        self
    }

    /// Clone a repository into `dir` with the configured options.
    ///
    /// Interactive — may prompt for credentials.
    pub fn clone(self, url: &str, dir: impl AsRef<Path>) -> anyhow::Result<Output> {
        let dir = dir.as_ref().to_string_lossy().to_string();
        let mut c = cmd("git").arg("clone");
        if let Some(branch) = &self.branch {
            c = c.arg("--branch").arg(branch);
        }
        match self.branches {
            Branches::All => {}
            Branches::Single => c = c.arg("--single-branch"),
            Branches::NoSingle => c = c.arg("--no-single-branch"),
        }
        if self.submodules {
            c = c.arg("--recurse-submodules");
        }
        c.args(&[url, &dir]).exec()
    }
}

/// Clone a repository into `dir` with git-default options.
///
/// Interactive — may prompt for credentials. See [`CloneOptions`] for
/// single-branch, branch checkout, all-branch, and submodule options.
pub fn clone(url: &str, dir: impl AsRef<Path>) -> anyhow::Result<Output> {
    CloneOptions::new().clone(url, dir)
}

/// Switch the current branch in `dir` via `git switch <branch>`.
///
/// Interactive — may prompt for credentials.
pub fn switch(dir: impl AsRef<Path>, branch: &str) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    cmd("git").args(&["-C", &dir, "switch", branch]).exec()
}

/// Create `branch` in `dir` and switch to it via `git switch -c <branch>`.
///
/// Interactive — may prompt for credentials.
pub fn switch_create(dir: impl AsRef<Path>, branch: &str) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    cmd("git")
        .args(&["-C", &dir, "switch", "-c", branch])
        .exec()
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
/// for use in a target's `check` function. A missing or non-repo `dir`, or
/// a git failure, reports `false` rather than an error — the repo simply
/// is not there yet.
pub fn is_git_repo(dir: impl AsRef<Path>) -> anyhow::Result<bool> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    let Ok(out) = cmd("git")
        .args(&["-C", &dir, "rev-parse", "--is-inside-work-tree"])
        .run()
    else {
        return Ok(false);
    };
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
