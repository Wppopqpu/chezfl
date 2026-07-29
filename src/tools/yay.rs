use crate::cmd::{cmd, run_cmd, Output};

/// Install packages via `yay -S --noconfirm` (supports AUR and official).
pub fn install(packages: &[&str]) -> anyhow::Result<Output> {
    let mut c = cmd("yay").args(&["-S", "--noconfirm"]);
    for pkg in packages {
        c = c.arg(pkg);
    }
    c.exec()
}

/// Remove packages via `yay -R --noconfirm`.
pub fn remove(packages: &[&str]) -> anyhow::Result<Output> {
    let mut c = cmd("yay").args(&["-R", "--noconfirm"]);
    for pkg in packages {
        c = c.arg(pkg);
    }
    c.exec()
}

/// Update all packages via `yay -Syu --noconfirm`.
pub fn update() -> anyhow::Result<Output> {
    cmd("yay").args(&["-Syu", "--noconfirm"]).exec()
}

/// Check whether a package is installed.
///
/// Suitable for use in a target's `check` function.
pub fn is_installed(package: &str) -> anyhow::Result<bool> {
    Ok(run_cmd("yay", &["-Qi", package]).is_ok())
}
