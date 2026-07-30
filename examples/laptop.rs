//! Example: Declare a laptop's desired state (macro API).
//!
//! Run with: `cargo run --example laptop [check|plan|apply]`
//!
//! This example uses the **macro API** to keep the config concise.
//!
//! Realistic `check_dep` patterns:
//! - A program must be installed before its config can be checked.
//! - Git must be available before a repo can be cloned.

use chezfl::{cmd::cmd, run, target, task, tools::yay};

fn main() -> anyhow::Result<()> {
    // ── ripgrep ──────────────────────────────────────────────────

    target!("rg_binary",
        description: "ripgrep (rg) binary exists",
        check: || yay::is_installed("ripgrep"),
    );

    // Can't check rg.conf if rg isn't installed
    target!("rg_conf",
        description: "rg.conf is present",
        check: || {
            let ok = cmd("test")
                .args(&["-f", "/etc/rg.conf"])
                .run()
                .is_ok();
            Ok(ok)
        },
        check_dep: [rg_binary],
    );

    // Aggregate: task installs rg → both leaf deps re-check
    target!("rg_ready",
        description: "ripgrep is installed and configured",
        depends_on: [rg_binary, rg_conf],
    );

    task!("install_rg",
        description: "Install ripgrep via yay",
        satisfies: [rg_ready],
        labels: ["install"],
        run: || {
            yay::install(&["ripgrep"])?;
            Ok(())
        },
    );

    // ── dotfiles ─────────────────────────────────────────────────

    target!("git_binary",
        description: "git binary exists",
        check: || {
            let ok = cmd("which").arg("git").run().is_ok();
            Ok(ok)
        },
    );

    // Can't clone if git isn't available
    target!("dotfiles_dir",
        description: "dotfiles repo directory exists",
        check: || {
            let ok = cmd("test")
                .args(&["-d", "/home/user/.dotfiles"])
                .run()
                .is_ok();
            Ok(ok)
        },
        check_dep: [git_binary],
    );

    target!("dotfiles_ready",
        description: "dotfiles repo is cloned",
        depends_on: [git_binary, dotfiles_dir],
    );

    task!("clone_dotfiles",
        description: "Clone dotfiles repo from GitHub",
        satisfies: [dotfiles_ready],
        depends_on: [git_binary],
        labels: ["clone"],
        run: || {
            cmd("git")
                .args(&[
                    "clone",
                    "https://github.com/user/dotfiles",
                    "/home/user/.dotfiles",
                ])
                .exec()?;
            Ok(())
        },
    );

    run!()
}
