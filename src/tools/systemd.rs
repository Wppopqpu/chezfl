use crate::cmd::{Cmd, Output, cmd};

/// Builder for `systemctl` invocations.
///
/// System units run via `sudo systemctl` — privilege escalation goes
/// through sudo rather than systemd's built-in polkit prompting. `--user`
/// units run directly on the calling user's systemd instance (no sudo).
///
/// Build once, run once: each command method consumes `self`.
///
/// # Example
///
/// ```no_run
/// use chezfl::tools::systemd::Systemctl;
/// // enable and start a system unit
/// Systemctl::new().now().enable("foo.service")?;
/// // check a user unit
/// let running = Systemctl::new().user().is_unit_running("foo.service")?;
/// # anyhow::Ok(())
/// ```
#[derive(Clone, Default)]
pub struct Systemctl {
    user: bool,
    now: bool,
}

impl Systemctl {
    /// Create a builder for system (sudo) systemctl invocations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Operate on the calling user's systemd instance (`--user`, no sudo).
    pub fn user(mut self) -> Self {
        self.user = true;
        self
    }

    /// Combine the action with `--now` so `enable`/`disable` also
    /// start/stop the unit immediately.
    pub fn now(mut self) -> Self {
        self.now = true;
        self
    }

    /// Enable a unit at boot via `systemctl enable <unit>`.
    pub fn enable(self, unit: &str) -> anyhow::Result<Output> {
        self.cmd("enable").arg(unit).exec()
    }

    /// Disable a unit at boot via `systemctl disable <unit>`.
    pub fn disable(self, unit: &str) -> anyhow::Result<Output> {
        self.cmd("disable").arg(unit).exec()
    }

    /// Reload systemd after unit files change via `systemctl daemon-reload`.
    pub fn daemon_reload(self) -> anyhow::Result<Output> {
        self.cmd("daemon-reload").exec()
    }

    /// Start a unit via `systemctl start <unit>`.
    pub fn start(self, unit: &str) -> anyhow::Result<Output> {
        self.cmd("start").arg(unit).exec()
    }

    /// Stop a unit via `systemctl stop <unit>`.
    pub fn stop(self, unit: &str) -> anyhow::Result<Output> {
        self.cmd("stop").arg(unit).exec()
    }

    /// Restart a unit via `systemctl restart <unit>`.
    pub fn restart(self, unit: &str) -> anyhow::Result<Output> {
        self.cmd("restart").arg(unit).exec()
    }

    /// Reload a unit's configuration via `systemctl reload <unit>`.
    pub fn reload(self, unit: &str) -> anyhow::Result<Output> {
        self.cmd("reload").arg(unit).exec()
    }

    /// Check whether a unit is active via
    /// `systemctl show --property=ActiveState <unit>`.
    ///
    /// Read-only, so no sudo is needed even for system units. Non-active
    /// or missing units report `false`. Suitable for use in a target's
    /// `check` function.
    pub fn is_unit_running(self, unit: &str) -> anyhow::Result<bool> {
        let c = if self.user {
            cmd("systemctl").arg("--user")
        } else {
            cmd("systemctl")
        };
        let Ok(out) = c.args(&["show", "--property=ActiveState", unit]).run() else {
            return Ok(false);
        };
        Ok(out.stdout.contains("ActiveState=active"))
    }

    fn cmd(self, subcommand: &str) -> Cmd {
        let mut c = if self.user {
            cmd("systemctl").arg("--user")
        } else {
            cmd("sudo").arg("systemctl")
        };
        c = c.arg(subcommand);
        if self.now {
            c = c.arg("--now");
        }
        c
    }
}

/// Enable a system unit at boot via `sudo systemctl enable <unit>`.
pub fn enable(unit: &str) -> anyhow::Result<Output> {
    Systemctl::new().enable(unit)
}

/// Enable and start a system unit via
/// `sudo systemctl enable --now <unit>`.
pub fn enable_now(unit: &str) -> anyhow::Result<Output> {
    Systemctl::new().now().enable(unit)
}

/// Disable a system unit via `sudo systemctl disable <unit>`.
pub fn disable(unit: &str) -> anyhow::Result<Output> {
    Systemctl::new().disable(unit)
}

/// Reload systemd after unit files change via `sudo systemctl daemon-reload`.
pub fn daemon_reload() -> anyhow::Result<Output> {
    Systemctl::new().daemon_reload()
}

/// Start a system unit via `sudo systemctl start <unit>`.
pub fn start(unit: &str) -> anyhow::Result<Output> {
    Systemctl::new().start(unit)
}

/// Stop a system unit via `sudo systemctl stop <unit>`.
pub fn stop(unit: &str) -> anyhow::Result<Output> {
    Systemctl::new().stop(unit)
}

/// Restart a system unit via `sudo systemctl restart <unit>`.
pub fn restart(unit: &str) -> anyhow::Result<Output> {
    Systemctl::new().restart(unit)
}

/// Check whether a system unit is running.
///
/// Non-interactive and read-only (no sudo) — suitable for use in a
/// target's `check` function.
pub fn is_unit_running(unit: &str) -> anyhow::Result<bool> {
    Systemctl::new().is_unit_running(unit)
}
