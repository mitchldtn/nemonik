//! What nemonik tells herdr from inside a herdr pane: the pane's resume command, the
//! agent's name, and new tabs for sessions.

use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::time::Duration;

/// herdr sets `HERDR_BIN_PATH` in its panes; `herdr` on PATH otherwise.
fn herdr() -> Command {
    let bin = std::env::var("HERDR_BIN_PATH")
        .ok()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "herdr".to_string());
    let mut cmd = Command::new(bin);
    // A running attach owns the terminal.
    cmd.stdin(Stdio::null());
    cmd
}

fn herdr_ok(args: &[&str]) -> bool {
    herdr()
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

const RETRIES: u32 = 20;
const RETRY_INTERVAL: Duration = Duration::from_millis(250);

/// In the background, reports `nemonik attach <session>` as this pane's resume command
/// and names the pane's herdr agent after the session. herdr refuses both until it has
/// detected the agent in the pane, so each is retried for a few seconds. Failures are
/// silent: herdr before 0.9.2 has no resume, and an agent name can be taken.
/// herdr's docs say a source must send a `report-agent` state report before its resume
/// command is accepted; herdr 0.9.5 also accepts it once it has detected the agent, and
/// this relies on that, since a state report would override herdr's screen detection.
pub fn report_session(session: &str, agent: &'static str) {
    let Some(pane) = std::env::var("HERDR_PANE_ID")
        .ok()
        .filter(|p| !p.is_empty())
    else {
        return;
    };
    let session = session.to_string();
    // herdr refuses a name outside its agent name rule (an older long session name).
    let mut named = crate::launch::check_session_name(&session).is_err();
    let seq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
        .to_string();
    std::thread::spawn(move || {
        let mut reported = false;
        for _ in 0..RETRIES {
            reported = reported
                || herdr_ok(&[
                    "pane",
                    "report-agent-session",
                    &pane,
                    "--source",
                    "nemonik",
                    "--agent",
                    agent,
                    "--seq",
                    &seq,
                    "--",
                    "nemonik",
                    "attach",
                    &session,
                ]);
            named = named || herdr_ok(&["agent", "rename", &pane, &session]);
            if reported && named {
                return;
            }
            std::thread::sleep(RETRY_INTERVAL);
        }
    });
}

pub enum TabFocus {
    Focused,
    Background,
}

fn tab_pane_id(created: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(created)
        .ok()?
        .pointer("/result/root_pane/pane_id")?
        .as_str()
        .map(str::to_string)
}

/// Opens a tab labeled `label` in the launching pane's workspace (`HERDR_WORKSPACE_ID`),
/// or herdr's active workspace when that is unset, running
/// `exec nemonik attach <session>`, so the tab closes when the attach ends.
pub fn open_tab(session: &str, label: &str, focus: TabFocus) -> Result<()> {
    let focus = match focus {
        TabFocus::Focused => "--focus",
        TabFocus::Background => "--no-focus",
    };
    let mut create = herdr();
    create.args(["tab", "create", "--label", label, focus]);
    if let Some(workspace) = std::env::var("HERDR_WORKSPACE_ID")
        .ok()
        .filter(|w| !w.is_empty())
    {
        create.args(["--workspace", &workspace]);
    }
    let created = create.output().context("failed to run herdr")?;
    let pane = tab_pane_id(&String::from_utf8_lossy(&created.stdout))
        .filter(|_| created.status.success())
        .ok_or_else(|| {
            let stdout = String::from_utf8_lossy(&created.stdout);
            let stderr = String::from_utf8_lossy(&created.stderr);
            anyhow::anyhow!(
                "herdr didn't create a tab: {}",
                format!("{} {}", stdout.trim(), stderr.trim()).trim()
            )
        })?;
    let ran = herdr()
        .args([
            "pane",
            "run",
            &pane,
            &format!("exec nemonik attach {session}"),
        ])
        .output()
        .context("failed to run herdr")?;
    if !ran.status.success() {
        anyhow::bail!(
            "herdr didn't run the attach in its new tab: {}",
            String::from_utf8_lossy(&ran.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_pane_id_reads_the_root_pane_of_a_created_tab() {
        let created = r#"{"id":"cli:tab:create","result":{"root_pane":{"pane_id":"w1:p2","tab_id":"w1:t2"},"tab":{"label":"probe","tab_id":"w1:t2"},"type":"tab_created"}}"#;
        assert_eq!(tab_pane_id(created).as_deref(), Some("w1:p2"));
        let refused = r#"{"error":{"code":"workspace_not_found","message":"no workspace"},"id":"cli:tab:create"}"#;
        assert_eq!(tab_pane_id(refused), None);
        assert_eq!(tab_pane_id("not json"), None);
    }
}
