//! Demonstrate the Cmd API: run, exec, timeout, retry.
//!
//! Run with: `cargo run --example cmd_demo`

use chezfl::cmd::{cmd, run_cmd};

fn main() -> anyhow::Result<()> {
    // ── captured execution ──────────────────────────────────────────
    println!("--- captured ---");

    let out = run_cmd("echo", &["hello", "chezfl"])?;
    println!("stdout: {}", out.stdout.trim());

    let out = cmd("sh").args(&["-c", "echo err >&2"]).run()?;
    println!("stderr: {}", out.stderr.trim());

    // ── non-zero exit ───────────────────────────────────────────────
    match cmd("false").run() {
        Err(e) => println!("expected error: {e:#}"),
        Ok(_) => unreachable!(),
    }

    // ── env and dir ─────────────────────────────────────────────────
    let out = cmd("sh")
        .args(&["-c", "echo $CHEZFL_DEMO"])
        .env("CHEZFL_DEMO", "works")
        .run()?;
    println!("env var: {}", out.stdout.trim());

    let tmp = std::env::temp_dir();
    let out = cmd("pwd").dir(&tmp).run()?;
    assert_eq!(out.stdout.trim(), tmp.to_string_lossy());
    println!("pwd matches temp dir ✓");

    // ── interactive execution ───────────────────────────────────────
    println!("--- interactive ---");
    let out = cmd("true").exec()?;
    println!("true exited successfully: {}", out.status.success());

    // ── retry ───────────────────────────────────────────────────────
    match cmd("false").retry(2).run() {
        Err(e) => println!("retry exhausted: {e:#}"),
        Ok(_) => unreachable!(),
    }

    // ── timeout (captured mode) ─────────────────────────────────────
    let out = cmd("true")
        .timeout(std::time::Duration::from_secs(10))
        .run()?;
    println!("true with 10s timeout: {}", out.status.success());

    // ── timeout fires ───────────────────────────────────────────────
    match cmd("sleep")
        .arg("10")
        .timeout(std::time::Duration::from_millis(10))
        .exec()
    {
        Err(e) => println!("timeout fired: {e:#}"),
        Ok(_) => unreachable!(),
    }

    Ok(())
}
