use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command as StdCommand, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// The result of a command execution.
///
/// For captured runs ([`Cmd::run`]) both `stdout` and `stderr` contain the
/// program output. For interactive runs ([`Cmd::exec`]) they are empty
/// since IO went directly to the terminal.
#[derive(Debug)]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Build and execute a system command.
///
/// Two execution modes:
/// - [`run`](Cmd::run) — captures stdout/stderr, stdin is null.
/// - [`exec`](Cmd::exec) — inherits stdin/stdout/stderr (fully interactive).
///
/// Supports environment variables, working directory, timeout, and retry.
///
/// # Examples
///
/// ```no_run
/// use chezfl::cmd::{cmd, run_cmd};
///
/// // Quick one-shot
/// let out = run_cmd("echo", &["hello"])?;
///
/// // Builder with options
/// let out = cmd("ping")
///     .arg("-c").arg("1")
///     .arg("example.com")
///     .timeout(std::time::Duration::from_secs(5))
///     .retry(2)
///     .run()?;
/// # anyhow::Ok(())
/// ```
#[derive(Debug, Clone)]
pub struct Cmd {
    program: String,
    args: Vec<String>,
    envs: HashMap<String, String>,
    dir: Option<PathBuf>,
    timeout: Option<Duration>,
    retry: usize,
}

/// One-shot captured execution: `run_cmd("git", &["status"])`.
pub fn run_cmd(program: &str, args: &[&str]) -> anyhow::Result<Output> {
    cmd(program).args(args).run()
}

/// Create a new [`Cmd`] builder.
pub fn cmd(program: &str) -> Cmd {
    Cmd::new(program)
}

impl Cmd {
    pub fn new(program: &str) -> Self {
        Cmd {
            program: program.to_string(),
            args: Vec::new(),
            envs: HashMap::new(),
            dir: None,
            timeout: None,
            retry: 0,
        }
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|a| a.to_string()));
        self
    }

    pub fn env(mut self, key: &str, val: &str) -> Self {
        self.envs.insert(key.to_string(), val.to_string());
        self
    }

    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = Some(dir.into());
        self
    }

    /// Set a timeout. The process is killed if it runs longer than `dur`.
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Retry up to `n` additional times on non-zero exit or timeout.
    pub fn retry(mut self, n: usize) -> Self {
        self.retry = n;
        self
    }

    /// Run with output captured (stdin null, stdout/stderr piped).
    pub fn run(self) -> anyhow::Result<Output> {
        self.exec_with_mode(false)
    }

    /// Run interactively (stdin/stdout/stderr inherited).
    pub fn exec(self) -> anyhow::Result<Output> {
        self.exec_with_mode(true)
    }

    // ── internal ─────────────────────────────────────────────────────

    fn exec_with_mode(&self, interactive: bool) -> anyhow::Result<Output> {
        let max_attempts = self.retry + 1;

        for attempt in 1..=max_attempts {
            match self.try_once(interactive) {
                Ok(output) if output.status.success() => return Ok(output),
                Ok(output) => {
                    let stderr = output.stderr.trim();
                    let msg = format!(
                        "`{} {}` failed (exit code {:?}){}",
                        self.program,
                        self.args.join(" "),
                        output.status.code(),
                        if stderr.is_empty() { String::new() } else { format!(": {stderr}") },
                    );
                    if attempt == max_attempts {
                        anyhow::bail!(msg);
                    }
                }
                Err(e) => {
                    if attempt == max_attempts {
                        return Err(e);
                    }
                }
            }
        }
        unreachable!()
    }

    fn try_once(&self, interactive: bool) -> anyhow::Result<Output> {
        match (self.timeout, interactive) {
            (Some(dur), false) => self.run_captured_with_timeout(dur),
            (Some(dur), true) => self.exec_with_timeout(dur),
            (None, _) => self.run_direct(interactive),
        }
    }

    fn build(&self, interactive: bool) -> StdCommand {
        let mut cmd = StdCommand::new(&self.program);
        cmd.args(&self.args);
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        if let Some(ref dir) = self.dir {
            cmd.current_dir(dir);
        }
        if interactive {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
        } else {
            cmd.stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        }
        cmd
    }

    /// No timeout — simplest path.
    fn run_direct(&self, interactive: bool) -> anyhow::Result<Output> {
        if interactive {
            let mut child = self.build(true).spawn()?;
            let status = child.wait()?;
            Ok(Output { status, stdout: String::new(), stderr: String::new() })
        } else {
            let std_output = self.build(false).output()?;
            Ok(Output {
                status: std_output.status,
                stdout: String::from_utf8_lossy(&std_output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&std_output.stderr).to_string(),
            })
        }
    }

    /// Captured mode with timeout — use `.output()` in a thread.
    fn run_captured_with_timeout(&self, dur: Duration) -> anyhow::Result<Output> {
        let mut cmd = self.build(false);
        let (tx, rx) = mpsc::channel::<std::io::Result<std::process::Output>>();

        thread::spawn(move || {
            tx.send(cmd.output()).ok();
        });

        match rx.recv_timeout(dur) {
            Ok(Ok(out)) => Ok(Output {
                status: out.status,
                stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            }),
            Ok(Err(e)) => Err(e.into()),
            Err(RecvTimeoutError::Timeout) => {
                anyhow::bail!(
                    "`{} {}` timed out after {dur:?}",
                    self.program,
                    self.args.join(" ")
                )
            }
            Err(RecvTimeoutError::Disconnected) => {
                anyhow::bail!("internal error: command thread disconnected")
            }
        }
    }

    /// Interactive mode with timeout — kill by PID.
    fn exec_with_timeout(&self, dur: Duration) -> anyhow::Result<Output> {
        let mut child = self.build(true).spawn()?;
        let pid = child.id();
        let (tx, rx) = mpsc::channel::<std::io::Result<ExitStatus>>();

        thread::spawn(move || {
            tx.send(child.wait()).ok();
        });

        match rx.recv_timeout(dur) {
            Ok(Ok(status)) => Ok(Output { status, stdout: String::new(), stderr: String::new() }),
            Ok(Err(e)) => Err(e.into()),
            Err(RecvTimeoutError::Timeout) => {
                kill_pid(pid);
                let _ = rx.recv_timeout(Duration::from_secs(5));
                anyhow::bail!(
                    "`{} {}` timed out after {dur:?}",
                    self.program,
                    self.args.join(" ")
                )
            }
            Err(RecvTimeoutError::Disconnected) => {
                anyhow::bail!("internal error: command thread disconnected")
            }
        }
    }
}

fn kill_pid(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg(pid.to_string())
        .status();
    thread::sleep(Duration::from_millis(200));
    let _ = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status();
}

// ── tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_captures_stdout() {
        let out = run_cmd("echo", &["hello chezfl"]).unwrap();
        assert!(out.status.success());
        assert_eq!(out.stdout.trim(), "hello chezfl");
    }

    #[test]
    fn test_run_captures_stderr() {
        let out = cmd("sh").args(&["-c", "echo err >&2"]).run().unwrap();
        assert_eq!(out.stderr.trim(), "err");
    }

    #[test]
    fn test_run_non_zero_exit() {
        let err = cmd("false").run().unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("false"), "error should name command: {msg}");
        assert!(msg.contains("exit code"), "error should mention code: {msg}");
    }

    #[test]
    fn test_env_var() {
        let out = cmd("sh")
            .args(&["-c", "echo $CHEZFL_TEST"])
            .env("CHEZFL_TEST", "works")
            .run()
            .unwrap();
        assert_eq!(out.stdout.trim(), "works");
    }

    #[test]
    fn test_dir() {
        let tmp = std::env::temp_dir();
        let out = cmd("pwd").dir(&tmp).run().unwrap();
        assert_eq!(out.stdout.trim(), tmp.to_string_lossy());
    }

    #[test]
    fn test_args_chaining() {
        let out = cmd("echo").arg("a").arg("b").args(&["c", "d"]).run().unwrap();
        assert_eq!(out.stdout.trim(), "a b c d");
    }

    #[test]
    fn test_exec_interactive_true() {
        let out = cmd("true").exec().unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn test_retry_all_fail() {
        let err = cmd("false").retry(2).run().unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("false"), "{msg}");
    }

    #[test]
    fn test_timeout_captured_trivial() {
        // Should complete well within timeout
        let out = cmd("true")
            .timeout(Duration::from_secs(10))
            .run()
            .unwrap();
        assert!(out.status.success());
    }

    #[test]
    fn test_timeout_interactive_trivial() {
        let out = cmd("true")
            .timeout(Duration::from_secs(10))
            .exec()
            .unwrap();
        assert!(out.status.success());
    }

    #[test]
    fn test_timeout_fires() {
        let err = cmd("sleep")
            .arg("10")
            .timeout(Duration::from_millis(10))
            .exec()
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("timed out"), "{msg}");
    }
}
