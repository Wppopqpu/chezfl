use std::path::Path;

use crate::cmd::{Output, cmd};
use anyhow::{Context, Result};

/// Stow a package into a target directory via `stow -S`.
///
/// Creates symlinks in `to` pointing back into `from/<package>`. The stow
/// directory (`from`) must contain a subdirectory named `package`.
///
/// Non-interactive — output is captured.
pub fn stow(from: impl AsRef<Path>, to: impl AsRef<Path>, package: &str) -> Result<Output> {
    run_stow("-S", "stow", from, to, package)
}

/// Unstow a package via `stow -D`.
///
/// Removes the symlinks previously created for `package` in `to`, folding
/// any directories left behind when possible.
///
/// Non-interactive — output is captured.
pub fn unstow(from: impl AsRef<Path>, to: impl AsRef<Path>, package: &str) -> Result<Output> {
    run_stow("-D", "unstow", from, to, package)
}

/// Restow a package via `stow -R`.
///
/// Unstows `package` and immediately stows it again. Useful to fold a
/// previously stowed package after its directory layout changed.
///
/// Non-interactive — output is captured.
pub fn restow(from: impl AsRef<Path>, to: impl AsRef<Path>, package: &str) -> Result<Output> {
    run_stow("-R", "restow", from, to, package)
}

/// Stow every package in the stow directory.
///
/// Each subdirectory of `from` is treated as a package and stowed into `to`.
/// Stops at the first failing package.
///
/// Non-interactive — output is captured.
pub fn stow_everything(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let from = from.as_ref();
    for entry in std::fs::read_dir(from)
        .with_context(|| format!("failed to read stow directory {}", from.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read stow directory {}", from.display()))?;
        if entry.file_type()?.is_dir() && !is_ignored(entry.file_name().to_str().unwrap_or("")) {
            let package = entry.file_name().to_string_lossy().into_owned();
            stow(from, &to, &package)?;
        }
    }
    Ok(())
}

/// Check whether a package is fully stowed into `to`.
///
/// A package is considered stowed when every top-level entry under
/// `from/<package>` is a symlink in `to` resolving to the corresponding
/// entry in the package tree.
///
/// Suitable for use in a target's `check` function. Returns `false` when
/// the package directory does not exist.
pub fn is_stowed(from: impl AsRef<Path>, to: impl AsRef<Path>, package: &str) -> Result<bool> {
    let src = from.as_ref().join(package);
    let dst = to.as_ref();
    is_stowed_impl(src, dst)
}

fn is_stowed_impl(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<bool> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let Ok(entries) = std::fs::read_dir(src) else {
        // target do not exists
        return Ok(false);
    };
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read package directory {}", src.display()))?;
        let target = dst.join(entry.file_name());

        if target.is_symlink() {
            if !symlink_resolves_to(&target, &entry.path()) {
                return Ok(false);
            }

            continue;
        }

        if target.is_dir() {
            if !is_stowed_impl(entry.path(), &target)? {
                return Ok(false);
            }
            continue;
        }
        return Ok(false);
    }
    Ok(true)
}

/// Check whether every package in the stow directory is stowed.
///
/// Each subdirectory of `from` is treated as a package and verified like
/// [`is_stowed`]. Suitable for use in a target's `check` function. Returns
/// `false` when the stow directory does not exist.
pub fn is_everything_stowed(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<bool> {
    let from = from.as_ref();
    let to = to.as_ref();
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(false);
    };
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read stow directory {}", from.display()))?;
        if entry.file_type()?.is_dir() && !is_ignored(entry.file_name().to_str().unwrap_or("")) {
            let package = entry.file_name().to_string_lossy().into_owned();
            if !is_stowed(from, to, &package)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn run_stow(
    flag: &str,
    verb: &str,
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
    package: &str,
) -> Result<Output> {
    let from = from.as_ref().to_string_lossy().to_string();
    let to = to.as_ref().to_string_lossy().to_string();
    cmd("stow")
        .args(&["-d", &from, "-t", &to, flag, package])
        .run()
        .with_context(|| format!("failed to {verb} package {package} (from {from} to {to})"))
}

fn symlink_resolves_to(link: &Path, source: &Path) -> bool {
    match (std::fs::canonicalize(link), std::fs::canonicalize(source)) {
        (Ok(link_real), Ok(source_real)) => link_real == source_real,
        _ => false,
    }
}

const IGNORED: &[&str] = &[".git"];
fn is_ignored(name: &str) -> bool {
    IGNORED.contains(&name)
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("chezfl_stow_test_{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) {
        std::os::unix::fs::symlink(src, dst).unwrap();
    }

    #[test]
    fn test_is_stowed() {
        let root = tmpdir("is_stowed");
        let from = root.join("from");
        let to = root.join("to");
        std::fs::create_dir_all(from.join("pkg")).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(from.join("pkg").join("a.txt"), "a").unwrap();
        std::fs::create_dir(from.join("pkg").join("sub")).unwrap();
        std::fs::write(from.join("pkg").join("sub").join("b.txt"), "b").unwrap();

        assert!(!is_stowed(&from, &to, "pkg").unwrap());

        symlink(from.join("pkg").join("a.txt"), to.join("a.txt"));
        symlink(from.join("pkg").join("sub"), to.join("sub"));
        assert!(is_stowed(&from, &to, "pkg").unwrap());

        std::fs::remove_file(to.join("a.txt")).unwrap();
        assert!(!is_stowed(&from, &to, "pkg").unwrap());

        assert!(!is_stowed(&from, &to, "missing").unwrap());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_is_everything_stowed() {
        let root = tmpdir("is_everything");
        let from = root.join("from");
        let to = root.join("to");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        for pkg in ["bash", "vim"] {
            let p = from.join(pkg);
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join(pkg), "x").unwrap();
        }

        assert!(!is_everything_stowed(&from, &to).unwrap());

        symlink(from.join("bash").join("bash"), to.join("bash"));
        assert!(!is_everything_stowed(&from, &to).unwrap());

        symlink(from.join("vim").join("vim"), to.join("vim"));
        assert!(is_everything_stowed(&from, &to).unwrap());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
