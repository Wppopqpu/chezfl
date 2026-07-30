use crate::cmd::{Output, cmd, run_cmd};
use crate::tools::fs;

/// Check whether `xdg-mime` is available at `/usr/bin/xdg-mime`.
pub fn is_available() -> anyhow::Result<bool> {
    fs::is_runnable("/usr/bin/xdg-mime")
}

/// Query the current default desktop entry for *mime_type*.
///
/// Returns `None` when `xdg-mime` is not available.
pub fn query_default(mime_type: &str) -> anyhow::Result<Option<String>> {
    if !is_available()? {
        return Ok(None);
    }
    let out = run_cmd("xdg-mime", &["query", "default", mime_type])?;
    let val = out.stdout.trim().to_string();
    Ok(Some(val))
}

/// Check whether the default desktop entry for *mime_type* matches *expected*.
///
/// Suitable for use in a target's `check` function. Returns `false` when
/// `xdg-mime` is not available.
pub fn is_default(mime_type: &str, expected: &str) -> anyhow::Result<bool> {
    let current = query_default(mime_type)?;
    match current {
        Some(val) => Ok(val == expected),
        None => Ok(false),
    }
}

/// Set the default desktop entry for *mime_type* to *desktop* via `xdg-mime default`.
///
/// Interactive — output is forwarded to the terminal.
pub fn set_default(mime_type: &str, desktop: &str) -> anyhow::Result<Output> {
    cmd("xdg-mime")
        .arg("default")
        .arg(desktop)
        .arg(mime_type)
        .exec()
}
