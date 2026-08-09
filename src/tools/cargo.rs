use std::path::Path;

use crate::cmd::{Output, cmd};

/// Install a crate from crates.io via `cargo install <package>`.
///
/// Installs into `$CARGO_HOME/bin` (default `~/.cargo/bin`). A newer
/// version is downloaded when one is available, so this also serves as an
/// update.
///
/// Interactive — prints build progress to the terminal.
pub fn install(package: &str) -> anyhow::Result<Output> {
    cmd("cargo").args(&["install", package]).exec()
}

/// Install a crate from a local source tree via `cargo install --path <dir>`.
///
/// Interactive — prints build progress to the terminal.
pub fn install_path(dir: impl AsRef<Path>) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    cmd("cargo").args(&["install", "--path", &dir]).exec()
}

/// Install a crate into a custom root via `cargo install --root <root>`.
///
/// The binary is placed in `<root>/bin`. Interactive — prints build
/// progress to the terminal.
pub fn install_root(package: &str, root: impl AsRef<Path>) -> anyhow::Result<Output> {
    let root = root.as_ref().to_string_lossy().to_string();
    cmd("cargo")
        .args(&["install", "--root", &root, package])
        .exec()
}

/// Install a crate from a local tree into a custom root via
/// `cargo install --path <dir> --root <root>`.
///
/// The binary is placed in `<root>/bin`. Interactive — prints build
/// progress to the terminal.
pub fn install_path_root(dir: impl AsRef<Path>, root: impl AsRef<Path>) -> anyhow::Result<Output> {
    let dir = dir.as_ref().to_string_lossy().to_string();
    let root = root.as_ref().to_string_lossy().to_string();
    cmd("cargo")
        .args(&["install", "--path", &dir, "--root", &root])
        .exec()
}

/// Install a crate from a git repository via `cargo install --git <url>`.
///
/// Interactive — prints build progress to the terminal.
pub fn install_git(url: &str) -> anyhow::Result<Output> {
    cmd("cargo").args(&["install", "--git", url]).exec()
}

/// Reinstall a crate, overwriting the existing installation via
/// `cargo install --force`.
///
/// Interactive — prints build progress to the terminal.
pub fn install_force(package: &str) -> anyhow::Result<Output> {
    cmd("cargo").args(&["install", "--force", package]).exec()
}

/// Check whether a crate is installed via `cargo install --list`.
///
/// Suitable for use in a target's `check` function. Non-interactive.
pub fn is_installed(package: &str) -> anyhow::Result<bool> {
    let out = cmd("cargo").args(&["install", "--list"]).run()?;
    Ok(out
        .stdout
        .lines()
        .any(|l| l.trim_start().starts_with(&format!("{package} v"))))
}
