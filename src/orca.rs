//! What nemonik tells Orca from inside an Orca terminal: the variables a session carries
//! and new terminals for sessions.

use crate::session::TabFocus;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::process::{Command, Stdio};

/// The variables that name one Orca terminal. Orca's status hooks report under
/// `ORCA_PANE_KEY` alone, so a session carries the identity of the terminal showing it.
const IDENTITY: [&str; 4] = [
    "ORCA_PANE_KEY",
    "ORCA_AGENT_PANE",
    "ORCA_TAB_ID",
    "ORCA_TERMINAL_HANDLE",
];

/// The launching pane's `ORCA_*` variables for a session's spec, with the pane identity
/// only when the session attaches in that pane (`Attach::Here`).
pub fn forwarded_env(
    vars: impl IntoIterator<Item = (String, String)>,
    attach: Attach,
) -> BTreeMap<String, String> {
    vars.into_iter()
        .filter(|(key, _)| key.starts_with("ORCA_"))
        .filter(|(key, _)| matches!(attach, Attach::Here) || !IDENTITY.contains(&key.as_str()))
        .collect()
}

pub enum Attach {
    Here,
    Elsewhere,
}

/// A terminal Orca created.
#[derive(Debug, PartialEq)]
pub struct Terminal {
    handle: String,
    pane_key: String,
    tab_id: String,
}

impl Terminal {
    /// The identity a session shown in this terminal carries, in `IDENTITY`'s order.
    pub fn identity(&self) -> BTreeMap<String, String> {
        IDENTITY
            .into_iter()
            .zip([&self.pane_key, &self.pane_key, &self.tab_id, &self.handle])
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }
}

fn created_terminal(created: &str) -> Option<Terminal> {
    let value = serde_json::from_str::<serde_json::Value>(created).ok()?;
    let terminal = value.pointer("/result/terminal")?;
    let field = |name: &str| terminal.get(name)?.as_str().map(str::to_string);
    Some(Terminal {
        handle: field("handle")?,
        pane_key: field("paneKey")?,
        tab_id: field("tabId")?,
    })
}

/// Orca's own words for a refusal: its first next step, else its error message.
fn refusal(output: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let error = value.get("error")?;
    error
        .pointer("/data/nextSteps/0")
        .or_else(|| error.get("message"))?
        .as_str()
        .map(str::to_string)
}

/// Orca sets `ORCA_CLI_BIN_DIR` in its terminals; `orca` on PATH otherwise.
fn orca() -> Command {
    let bin = std::env::var("ORCA_CLI_BIN_DIR")
        .ok()
        .filter(|dir| !dir.is_empty())
        .map(|dir| format!("{dir}/orca"))
        .unwrap_or_else(|| "orca".to_string());
    let mut cmd = Command::new(bin);
    // A running attach owns the terminal.
    cmd.stdin(Stdio::null());
    cmd
}

/// Creates a terminal titled `label` in the launching worktree (`ORCA_WORKTREE_ID`), or
/// Orca's current one when that is unset. It runs nothing yet: the session it will show
/// must start with its identity.
pub fn create_terminal(label: &str, focus: TabFocus) -> Result<Terminal> {
    let mut create = orca();
    create.args(["terminal", "create", "--title", label, "--json"]);
    if matches!(focus, TabFocus::Focused) {
        create.arg("--focus");
    }
    if let Some(worktree) = std::env::var("ORCA_WORKTREE_ID")
        .ok()
        .filter(|w| !w.is_empty())
    {
        create.args(["--worktree", &format!("id:{worktree}")]);
    }
    let created = create.output().context("failed to run orca")?;
    created_terminal(&String::from_utf8_lossy(&created.stdout))
        .filter(|_| created.status.success())
        .ok_or_else(|| {
            let stdout = String::from_utf8_lossy(&created.stdout);
            let stderr = String::from_utf8_lossy(&created.stderr);
            let reason =
                refusal(&stdout).unwrap_or_else(|| format!("{} {}", stdout.trim(), stderr.trim()));
            anyhow::anyhow!("Orca didn't create a terminal: {}", reason.trim())
        })
}

/// Runs `exec nemonik attach <session>` in `terminal`, so it closes when the attach ends.
pub fn run_attach(terminal: &Terminal, session: &str) -> Result<()> {
    let sent = orca()
        .args([
            "terminal",
            "send",
            "--terminal",
            &terminal.handle,
            "--text",
            &format!("exec nemonik attach {session}"),
            "--enter",
        ])
        .output()
        .context("failed to run orca")?;
    if !sent.status.success() {
        anyhow::bail!(
            "Orca didn't run the attach in its new terminal: {}",
            String::from_utf8_lossy(&sent.stderr).trim()
        );
    }
    Ok(())
}

/// Best-effort: closes a terminal whose session never started.
pub fn close(terminal: &Terminal) {
    let _ = orca()
        .args(["terminal", "close", "--terminal", &terminal.handle])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane_vars() -> Vec<(String, String)> {
        [
            ("ORCA_PANE_KEY", "t1:l1"),
            ("ORCA_AGENT_PANE", "t1:l1"),
            ("ORCA_TAB_ID", "t1"),
            ("ORCA_TERMINAL_HANDLE", "term_1"),
            ("ORCA_WORKTREE_ID", "r::/p"),
            ("ORCA_AGENT_HOOK_ENDPOINT", "/e.env"),
            ("TERM_PROGRAM", "Orca"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    #[test]
    fn forwarded_env_keeps_the_pane_identity_only_for_an_attach_here() {
        let here = forwarded_env(pane_vars(), Attach::Here);
        assert_eq!(here.len(), 6, "{here:?}");
        assert!(!here.contains_key("TERM_PROGRAM"));
        let elsewhere = forwarded_env(pane_vars(), Attach::Elsewhere);
        assert_eq!(
            elsewhere.keys().collect::<Vec<_>>(),
            ["ORCA_AGENT_HOOK_ENDPOINT", "ORCA_WORKTREE_ID"]
        );
    }

    #[test]
    fn created_terminal_reads_the_identity_of_a_created_terminal() {
        let created = r#"{"id":"x","ok":true,"result":{"terminal":{"processId":1,"handle":"term_9","tabId":"t9","paneKey":"t9:l9","worktreeId":"r::/p","title":"k"}}}"#;
        let terminal = created_terminal(created).unwrap();
        let identity = terminal.identity();
        assert_eq!(identity["ORCA_PANE_KEY"], "t9:l9");
        assert_eq!(identity["ORCA_AGENT_PANE"], "t9:l9");
        assert_eq!(identity["ORCA_TAB_ID"], "t9");
        assert_eq!(identity["ORCA_TERMINAL_HANDLE"], "term_9");
        let refused =
            r#"{"id":"x","ok":false,"error":{"code":"worktree_not_found","message":"no"}}"#;
        assert_eq!(created_terminal(refused), None);
        assert_eq!(refusal(refused).as_deref(), Some("no"));
        let with_steps = r#"{"ok":false,"error":{"code":"selector_not_found","message":"selector_not_found","data":{"nextSteps":["No Orca workspace matched.","List them."]}}}"#;
        assert_eq!(
            refusal(with_steps).as_deref(),
            Some("No Orca workspace matched.")
        );
        assert_eq!(created_terminal("not json"), None);
        assert_eq!(refusal("not json"), None);
    }
}
