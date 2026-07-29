use crate::cmd::{Output, cmd, run_cmd};

/// Install packages via `yay -S` (supports AUR and official).
///
/// Interactive — prompts for confirmation and sudo when needed.
pub fn install(packages: &[&str]) -> anyhow::Result<Output> {
    let mut c = cmd("yay").arg("-S");
    for pkg in packages {
        c = c.arg(pkg);
    }
    c.exec()
}

/// Remove packages via `yay -R`.
///
/// Interactive — prompts for confirmation. Does not remove dependencies.
pub fn remove(packages: &[&str]) -> anyhow::Result<Output> {
    let mut c = cmd("yay").arg("-R");
    for pkg in packages {
        c = c.arg(pkg);
    }
    c.exec()
}

/// Remove packages and their dependencies via `yay -Rs`.
///
/// Interactive — prompts for confirmation. Recursively removes
/// dependencies that are not required by other packages.
pub fn remove_recursive(packages: &[&str]) -> anyhow::Result<Output> {
    let mut c = cmd("yay").arg("-Rs");
    for pkg in packages {
        c = c.arg(pkg);
    }
    c.exec()
}

/// Update all packages via `yay -Syu`.
///
/// Interactive — prompts for confirmation and sudo when needed.
pub fn update() -> anyhow::Result<Output> {
    cmd("yay").arg("-Syu").exec()
}

/// Check whether a package is installed.
///
/// Suitable for use in a target's `check` function. Non-interactive.
pub fn is_installed(package: &str) -> anyhow::Result<bool> {
    Ok(run_cmd("yay", &["-Qi", package]).is_ok())
}
