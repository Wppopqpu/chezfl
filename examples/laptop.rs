//! Example: Declare a laptop's desired state (macro API).
//!
//! Run with: `cargo run --example laptop [check|plan|apply]`
//!
//! This example uses the **macro API** to keep the config concise.

use chezfl::{cmd::cmd, target, task, run, tools::yay};

fn main() -> anyhow::Result<()> {
    target!("network");

    target!("rg_installed",
        check: || yay::is_installed("ripgrep"),
        depends_on: [network],
    );

    target!("dotfiles_repo",
        check: || {
            let ok = cmd("test")
                .args(&["-d", "/home/user/.dotfiles"])
                .run()
                .is_ok();
            Ok(ok)
        },
        depends_on: [network],
    );

    task!("install_rg",
        satisfies: [rg_installed],
        depends_on: [network],
        labels: ["install"],
        run: || {
            yay::install(&["ripgrep"])?;
            Ok(())
        },
    );

    task!("clone_dotfiles",
        satisfies: [dotfiles_repo],
        depends_on: [network],
        labels: ["clone"],
        run: || {
            cmd("git")
                .args(&["clone", "https://github.com/user/dotfiles", "/home/user/.dotfiles"])
                .exec()?;
            Ok(())
        },
    );

    run!()
}
