//! Starts and recovers the remote-control daemon's `<session>-daemon` tmux session.

use anyhow::{Context, Result};
use std::process::Command;

fn daemon_session_name(session: &str) -> String {
    format!("{session}-daemon")
}

fn daemon_already_running_message(session: &str, daemon_name: &str) -> String {
    format!(
        "a remote-control daemon is already running for '{session}' ('{daemon_name}').\n  \
         Attach to it with: nemonik attach {daemon_name}"
    )
}

fn harness_daemon_args(session: &str, project: &str) -> Vec<String> {
    vec![
        "__harness-daemon".to_string(),
        session.to_string(),
        "--project".to_string(),
        project.to_string(),
    ]
}

/// tmux starts the daemon with the tmux server's env, not ours, so the resolved
/// domain is always set on its command; it never picks up tmux's `NEMONIK_API`.
fn daemon_env_prefix(domain: &str) -> Vec<String> {
    vec![
        "env".to_string(),
        format!("{}={domain}", nemonik_core::config::API_ENV_VAR),
    ]
}

fn start_daemon_session(project: &str, primary_session_name: &str) -> Result<()> {
    let daemon_name = daemon_session_name(primary_session_name);
    let self_exe =
        std::env::current_exe().context("failed to resolve nemonik's own executable path")?;
    let domain = nemonik_core::config::ApiConfig::current().domain();

    let status = Command::new("tmux")
        .arg("new-session")
        .arg("-d")
        .arg("-s")
        .arg(&daemon_name)
        .arg("--")
        .args(daemon_env_prefix(domain))
        .arg(self_exe)
        .args(harness_daemon_args(primary_session_name, project))
        .status()
        .context("failed to start the harness daemon's tmux session")?;

    if !status.success() {
        // A concurrent launch may have created it since the check; report it the same way.
        if nemonik_core::tmux::tmux_has_session_blocking(&daemon_name) {
            anyhow::bail!(
                "{}",
                daemon_already_running_message(primary_session_name, &daemon_name)
            );
        }
        anyhow::bail!("tmux failed to start harness daemon session '{daemon_name}'");
    }

    println!(
        "Started remote-control daemon '{daemon_name}' (kill with: tmux kill-session -t {daemon_name})."
    );
    Ok(())
}

fn validate_daemon_start(session: &str, primary_exists: bool, daemon_exists: bool) -> Result<()> {
    if !primary_exists {
        anyhow::bail!(
            "no such session: '{session}'.\n  \
             Start one with `nemonik run` first, then retry `nemonik remote {session} --project <project>`."
        );
    }
    if daemon_exists {
        anyhow::bail!(
            "{}",
            daemon_already_running_message(session, &daemon_session_name(session))
        );
    }
    Ok(())
}

/// `nemonik remote`: starts remote control for a running session, launched
/// without `--remote` or whose daemon session died (the daemon only reconnects its
/// network link, not its own tmux session).
pub fn daemon(project: &str, session: &str) -> Result<()> {
    let daemon_name = daemon_session_name(session);
    let primary_exists = nemonik_core::tmux::tmux_has_session_blocking(session);
    let daemon_exists = nemonik_core::tmux::tmux_has_session_blocking(&daemon_name);
    validate_daemon_start(session, primary_exists, daemon_exists)?;
    start_daemon_session(project, session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_daemon_args_run_the_daemon_for_the_session_and_project() {
        match crate::parse(&harness_daemon_args("sess", "proj")) {
            crate::Command::HarnessDaemon { session, project } => {
                assert_eq!((session.as_str(), project.as_str()), ("sess", "proj"));
            }
            _ => panic!("the daemon session must run `__harness-daemon`"),
        }
    }

    #[test]
    fn validate_daemon_start_errors_when_primary_session_missing() {
        assert!(validate_daemon_start("sess", false, false).is_err());
    }

    #[test]
    fn validate_daemon_start_errors_when_daemon_already_running() {
        assert!(validate_daemon_start("sess", true, true).is_err());
    }

    #[test]
    fn validate_daemon_start_passes_when_primary_exists_and_daemon_does_not() {
        assert!(validate_daemon_start("sess", true, false).is_ok());
    }
}
