//! tmux primitives shared by the session commands and the remote daemon: session
//! checks and pane driving.

use anyhow::{Context, Result};
use std::io::Write as _;
use std::process::Command;

/// `=<session>:` target. `=` defeats tmux prefix matching (`s` vs `s-daemon`); the
/// trailing `:` is required for pane commands.
pub fn exact_target(session: &str) -> String {
    format!("={session}:")
}

/// Whether a session exists (`.output()` keeps tmux's error text off stderr).
pub fn tmux_has_session_blocking(session: &str) -> bool {
    Command::new("tmux")
        .arg("has-session")
        .arg("-t")
        .arg(exact_target(session))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// The text goes in on stdin, then is pasted as a bracketed paste (`-p`) so the app sees
// where it ends; `-r` keeps its line breaks as LF, `-d` deletes the buffer.
fn paste_argv(session: &str, buffer: &str) -> [Vec<String>; 3] {
    let target = exact_target(session);
    let argv = |args: &[&str]| args.iter().map(|a| a.to_string()).collect();
    [
        argv(&["load-buffer", "-b", buffer, "-"]),
        argv(&[
            "paste-buffer",
            "-p",
            "-d",
            "-r",
            "-b",
            buffer,
            "-t",
            &target,
        ]),
        argv(&["send-keys", "-t", &target, "Enter"]),
    ]
}

/// Pastes `command` into `session`'s pane and submits it.
pub fn tmux_submit(session: &str, command: &str) -> Result<()> {
    let buffer = format!("nemonik-{}", std::process::id());
    let [load, paste, enter] = paste_argv(session, &buffer);
    let mut steps = vec![enter];

    // tmux creates no buffer from empty input, so empty text is just Enter.
    if !command.is_empty() {
        let mut child = Command::new("tmux")
            .args(&load)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("failed to run tmux load-buffer")?;
        let written = child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(command.as_bytes());
        let status = child
            .wait()
            .context("failed to wait for tmux load-buffer")?;
        written.context("failed to write to tmux load-buffer")?;
        if !status.success() {
            anyhow::bail!("tmux load-buffer failed for session '{session}'");
        }
        steps.insert(0, paste);
    }

    for args in steps {
        let status = Command::new("tmux")
            .args(&args)
            .status()
            .with_context(|| format!("failed to run tmux {}", args[0]))?;
        if !status.success() {
            anyhow::bail!("tmux {} failed for session '{session}'", args[0]);
        }
    }
    Ok(())
}

/// `session`'s pane text. `scrollback_lines` adds that many lines of history above
/// the visible pane.
pub fn tmux_capture_pane(session: &str, scrollback_lines: Option<u64>) -> Result<String> {
    let mut cmd = Command::new("tmux");
    cmd.arg("capture-pane")
        .arg("-t")
        .arg(exact_target(session))
        .arg("-p");
    if let Some(n) = scrollback_lines {
        cmd.arg("-S").arg(format!("-{n}"));
    }
    let output = cmd.output().context("failed to run tmux capture-pane")?;
    if !output.status.success() {
        anyhow::bail!("tmux capture-pane failed for session '{session}'");
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
