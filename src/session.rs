//! The detached tmux session lifecycle: create, materialize skills, run the agent,
//! clean up, attach.

use anyhow::{Context, Result};
use nemonik_core::launchspec::{
    LaunchSpec, consume_launch_spec, remove_launch_spec, write_launch_spec,
};
use nemonik_core::tmux::tmux_has_session_blocking;
use std::path::Path;
use std::process::Command;

/// Makes the terminal tab show `<session> - <template>`, scoped to this session only.
fn window_title_commands(session: &str) -> Vec<Vec<String>> {
    vec![
        // Off by default; without it the tab just says "tmux".
        vec![
            "set-option".to_string(),
            "-t".to_string(),
            session.to_string(),
            "set-titles".to_string(),
            "on".to_string(),
        ],
        vec![
            "set-option".to_string(),
            "-t".to_string(),
            session.to_string(),
            "set-titles-string".to_string(),
            "#S - #W".to_string(),
        ],
    ]
}

/// Applies the title commands; cosmetic, so failures are non-fatal.
fn apply_window_title(session: &str) {
    for cmd_args in window_title_commands(session) {
        match Command::new("tmux").args(&cmd_args).status() {
            Ok(status) if status.success() => {}
            _ => {
                eprintln!(
                    "warning: failed to set the tmux window title for session '{session}' \
                     (cosmetic only, the session is still running)"
                );
                break;
            }
        }
    }
}

/// The detached `new-session` argv. The agent argv stays in the spec file: tmux's
/// command-length limit is small, and it would expose secrets.
fn detached_tmux_args(session: &str, template: &str, exe: &str, spec_path: &str) -> Vec<String> {
    vec![
        "new-session".to_string(),
        "-d".to_string(),
        "-s".to_string(),
        session.to_string(),
        "-n".to_string(),
        template.to_string(),
        "--".to_string(),
        exe.to_string(),
        "__exec-agent".to_string(),
        spec_path.to_string(),
    ]
}

/// A specific slot, to avoid clobbering another tool's hook at the default `[0]`.
const SESSION_CLOSED_HOOK_SLOT: &str = "session-closed[50]";

/// The global `session-closed` hook body (it only fires globally). Uses an unquoted
/// `#{q:hook_session_name}`: the bare form is a shell injection, since tmux expands any
/// closing session's name unescaped. `None` if the exe path can't be safely quoted.
fn session_closed_hook_command(exe: &Path) -> Option<String> {
    let exe_str = exe.to_str()?;
    if exe_str.contains('\'') {
        return None;
    }
    Some(format!(
        "run-shell \"'{exe_str}' cleanup-skills #{{q:hook_session_name}}\""
    ))
}

/// Best-effort and idempotent; `cleanup-skills` is the manual fallback.
fn register_session_closed_hook(exe: &Path) {
    let Some(command) = session_closed_hook_command(exe) else {
        eprintln!(
            "warning: nemonik's executable path can't be safely embedded in a tmux hook; \
             materialized skills won't be auto-cleaned up when this session closes (run \
             `nemonik cleanup-skills <session>` manually if needed)"
        );
        return;
    };

    match Command::new("tmux")
        .arg("set-hook")
        .arg("-g")
        .arg(SESSION_CLOSED_HOOK_SLOT)
        .arg(&command)
        .status()
    {
        Ok(status) if status.success() => {}
        _ => eprintln!(
            "warning: failed to register the tmux session-closed cleanup hook; materialized \
             skills won't be auto-cleaned up when this session closes (run `nemonik \
             cleanup-skills <session>` manually if needed)"
        ),
    }
}

/// Substitutes `{session}` into the `--new` template. Rejects anything outside
/// `[A-Za-z0-9._-]` (it reaches a shell) and adds no `=` (zsh would expand it).
fn build_new_window_command(template: &str, session: &str) -> Result<String> {
    if !session
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        anyhow::bail!(
            "session name '{session}' has characters unsafe to substitute into a shell \
             command; refusing to build a --new window command for it (the session is \
             running; attach with: nemonik attach {session})"
        );
    }
    Ok(template.replace("{session}", session))
}

/// Runs `new_window_command` (with `{session}` filled in) via `$SHELL -c`.
fn open_new_window(session: &str) -> Result<()> {
    let template = nemonik_core::config::require_new_window_command()?;
    let command = build_new_window_command(&template, session)?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let status = Command::new(&shell)
        .arg("-c")
        .arg(&command)
        .status()
        .with_context(|| format!("failed to run the --new window command via {shell}"))?;
    if !status.success() {
        anyhow::bail!(
            "the --new window command exited non-zero (session '{session}' is still running; \
             attach with: nemonik attach {session})"
        );
    }
    Ok(())
}

/// The `tmux new-session` spawn error, with install instructions when tmux is missing.
fn tmux_spawn_error(e: std::io::Error) -> anyhow::Error {
    if e.kind() == std::io::ErrorKind::NotFound {
        anyhow::anyhow!(
            "nemonik needs tmux, which isn't installed or isn't on PATH. Install it:\n  \
             macOS:          brew install tmux\n  \
             Debian/Ubuntu:  sudo apt install tmux\n  \
             Fedora/RHEL:    sudo dnf install tmux\n  \
             Arch:           sudo pacman -S tmux"
        )
    } else {
        anyhow::Error::new(e).context("failed to start tmux session")
    }
}

/// The tmux user option that records which agent a session runs.
const AGENT_OPTION: &str = "@nemonik-agent";

/// Best-effort: without the record, a later attach inside herdr sets no agent hint.
fn record_agent(session: &str, agent: &str) {
    let recorded = Command::new("tmux")
        .args([
            "set-option",
            "-t",
            &nemonik_core::tmux::exact_target(session),
            AGENT_OPTION,
            agent,
        ])
        .status()
        .is_ok_and(|s| s.success());
    if !recorded {
        eprintln!(
            "warning: failed to record the agent on tmux session '{session}' (herdr won't \
             show its status when attached with `nemonik attach`)"
        );
    }
}

/// herdr's label for an agent nemonik launches.
fn agent_label(program: &str) -> Option<&'static str> {
    match program {
        "claude" => Some("claude"),
        "codex" => Some("codex"),
        _ => None,
    }
}

/// The agent recorded on `session`; `None` for a session with no record (a `-daemon`
/// session, or one started by an older nemonik).
fn recorded_agent(session: &str) -> Option<&'static str> {
    let output = Command::new("tmux")
        .args([
            "show-options",
            "-v",
            "-t",
            &nemonik_core::tmux::exact_target(session),
            AGENT_OPTION,
        ])
        .output()
        .ok()?;
    agent_label(String::from_utf8_lossy(&output.stdout).trim())
}

fn attach_subcommand(in_tmux: bool) -> &'static str {
    if in_tmux { "switch-client" } else { "attach" }
}

/// `switch-client` inside tmux, else `attach`. Returns the raw status. With `herdr_agent`
/// (only outside tmux), the attach carries `HERDR_AGENT` so herdr shows the agent behind
/// it, and herdr is told how to resume the session.
fn attach_or_switch(
    session: &str,
    herdr_agent: Option<&'static str>,
) -> std::io::Result<std::process::ExitStatus> {
    let in_tmux = std::env::var_os("TMUX").is_some();
    let mut cmd = Command::new("tmux");
    cmd.arg(attach_subcommand(in_tmux))
        .arg("-t")
        // `=` blocks tmux prefix matching (`s` would also match `s-daemon`).
        .arg(format!("={session}"));
    match herdr_agent.filter(|_| !in_tmux) {
        Some(agent) => {
            let mut child = cmd.env("HERDR_AGENT", agent).spawn()?;
            crate::herdr::report_session(session, agent);
            child.wait()
        }
        None => cmd.status(),
    }
}

pub fn attach(session: &str) -> Result<()> {
    // Report "missing" and "couldn't attach" separately.
    if !tmux_has_session_blocking(session) {
        anyhow::bail!("no such session: {session}");
    }
    let herdr_agent = if nemonik_core::config::herdr_active()? {
        recorded_agent(session)
    } else {
        None
    };
    let status = attach_or_switch(session, herdr_agent)?;
    if !status.success() {
        anyhow::bail!(
            "failed to attach to session '{session}' (it exists). If you're inside tmux, \
             detach first, or run `tmux switch-client -t ={session}`."
        );
    }
    Ok(())
}

pub fn write(session: &str, command: &str) -> Result<()> {
    if !tmux_has_session_blocking(session) {
        anyhow::bail!("no such session: {session}");
    }
    nemonik_core::tmux::tmux_submit(session, command)
}

pub fn read(session: &str, scrollback_lines: Option<u64>) -> Result<()> {
    if !tmux_has_session_blocking(session) {
        anyhow::bail!("no such session: {session}");
    }
    let pane = nemonik_core::tmux::tmux_capture_pane(session, scrollback_lines)?;
    print!("{pane}");
    Ok(())
}

/// Consumes the spec (read + unlink first) and `exec`s the agent; argv here is bounded
/// only by `ARG_MAX`, not tmux's limit.
pub fn exec_agent(spec_path: &str) -> Result<()> {
    let spec = consume_launch_spec(spec_path)?;
    use std::os::unix::process::CommandExt;
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args);
    // herdr's variables come only from the spec: the tmux server's environment can hold
    // those of the herdr pane it was first started from, including a stale pane id.
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("HERDR_") {
            cmd.env_remove(key);
        }
    }
    // `CODEX_HOME` for a Codex launch with skills, herdr's variables inside herdr.
    cmd.envs(&spec.env);
    Err(cmd.exec()).with_context(|| format!("failed to exec agent process '{}'", spec.program))
}

/// Creates the session, then starts remote control, opens a window, or attaches. Skills
/// must exist before `new-session`: Claude Code scans them at startup. Inside herdr
/// (`herdr`), `--new` and `--no-attach` open a herdr tab instead.
pub fn launch(
    spec: &LaunchSpec,
    no_attach: bool,
    new: bool,
    remote_project: Option<&str>,
    herdr: bool,
) -> Result<()> {
    let name = spec.session.as_str();
    let exe = std::env::current_exe().context("failed to resolve nemonik's own executable path")?;
    let exe_str = exe
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("nemonik's own executable path is not valid UTF-8"))?;

    // Checked again here, after the trust review: writing and cleaning up the skills of a
    // name another session took meanwhile would delete that session's skills.
    if tmux_has_session_blocking(name) {
        anyhow::bail!("session name '{name}' is taken: tmux already has a session by that name");
    }
    let spec_path = write_launch_spec(spec)?;
    // On failure, clean up the skills and spec: no session exists to fire the hook.
    let cleanup_partial = || {
        remove_launch_spec(name);
        if !spec.skills.is_empty() {
            let _ = nemonik_core::skills::cleanup(name);
        }
    };
    let Some(spec_path) = spec_path.to_str() else {
        cleanup_partial();
        anyhow::bail!(
            "launch spec path is not valid UTF-8: {}",
            spec_path.display()
        );
    };
    if !spec.skills.is_empty()
        && let Err(e) = nemonik_core::skills::materialize(&spec.skills, name, spec.skills_layout)
    {
        cleanup_partial();
        return Err(e);
    }

    match Command::new("tmux")
        .args(detached_tmux_args(name, &spec.template, exe_str, spec_path))
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(_) => {
            cleanup_partial();
            anyhow::bail!("tmux failed to start session '{name}'");
        }
        Err(e) => {
            cleanup_partial();
            return Err(tmux_spawn_error(e));
        }
    }

    apply_window_title(name);
    register_session_closed_hook(&exe);
    let agent = agent_label(&spec.program);
    record_agent(name, &spec.program);

    println!("Started detached session '{name}'.");

    if let Some(project) = remote_project {
        crate::remote::spawn::daemon(project, name)?;
    }

    if herdr && (new || no_attach) {
        use crate::herdr::TabFocus;
        let focus = if new {
            TabFocus::Focused
        } else {
            TabFocus::Background
        };
        match crate::herdr::open_tab(name, &spec.template, focus) {
            Ok(()) if new => println!("Opened in a herdr tab."),
            Ok(()) => println!("Opened in a background herdr tab."),
            Err(e) => eprintln!("nemonik: {e:#}"),
        }
        println!("Attach with: nemonik attach {name}");
        return Ok(());
    }
    if new {
        open_new_window(name)?;
        println!("Opened in a new window; attach elsewhere with: nemonik attach {name}");
        return Ok(());
    }
    if no_attach {
        println!("Attach with: nemonik attach {name}");
        return Ok(());
    }
    // The session exists, so an attach failure falls back to a hint, not an error.
    match attach_or_switch(name, agent.filter(|_| herdr)) {
        Ok(status) if status.success() => Ok(()),
        _ => {
            println!("Attach with: nemonik attach {name}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmux_spawn_error_gives_install_instructions_when_tmux_is_missing() {
        let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
        let msg = tmux_spawn_error(missing).to_string();
        assert!(msg.contains("needs tmux"), "{msg}");
        assert!(msg.contains("brew install tmux"), "{msg}");
        assert!(msg.contains("sudo apt install tmux"), "{msg}");

        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let msg = format!("{:#}", tmux_spawn_error(denied));
        assert!(msg.starts_with("failed to start tmux session"), "{msg}");
        assert!(!msg.contains("brew install"), "{msg}");
    }

    #[test]
    fn launch_removes_the_launch_spec_when_writing_skills_fails() {
        let spec = LaunchSpec {
            program: "claude".to_string(),
            args: vec![],
            session: format!("nemonik-test-{}-bad-skill", std::process::id()),
            template: "k".to_string(),
            skills: vec![nemonik_core::mcp::SkillSpec {
                name: "../x".to_string(),
                description: String::new(),
                body: String::new(),
            }],
            env: Default::default(),
            skills_layout: Default::default(),
        };
        assert!(launch(&spec, true, false, None, false).is_err());
        assert!(!nemonik_core::launchspec::launch_spec_path(&spec.session).exists());
    }

    #[test]
    fn detached_tmux_args_keep_the_agent_argv_off_the_command_line() {
        let args = detached_tmux_args(
            "nemonik-proj-code:review-abc123",
            "code:review",
            "/usr/local/bin/nemonik",
            "/tmp/nemonik-launch-x.json",
        );
        assert_eq!(
            args,
            vec![
                "new-session",
                "-d",
                "-s",
                "nemonik-proj-code:review-abc123",
                "-n",
                "code:review",
                "--",
                "/usr/local/bin/nemonik",
                "__exec-agent",
                "/tmp/nemonik-launch-x.json",
            ]
        );
        match crate::parse(&args[8..]) {
            crate::Command::ExecAgent { spec } => assert_eq!(spec, "/tmp/nemonik-launch-x.json"),
            _ => panic!("the session command must run `__exec-agent`"),
        }
    }

    #[test]
    fn build_new_window_command_substitutes_every_session_placeholder() {
        let out = build_new_window_command(
            "wezterm cli spawn --window-title {session} -- tmux attach -t ={session}",
            "nemonik-proj-code-review-abc123",
        )
        .unwrap();
        assert_eq!(
            out,
            "wezterm cli spawn --window-title nemonik-proj-code-review-abc123 -- tmux attach -t =nemonik-proj-code-review-abc123"
        );
    }

    #[test]
    fn build_new_window_command_rejects_a_session_name_with_shell_metacharacters() {
        assert!(build_new_window_command("echo {session}", "a; rm -rf ~").is_err());
        assert!(build_new_window_command("echo {session}", "a$(whoami)").is_err());
        assert!(build_new_window_command("echo {session}", "a`id`").is_err());
    }

    #[test]
    fn session_closed_hook_command_embeds_the_exe_path_and_the_quoted_session_format_variable() {
        let cmd = session_closed_hook_command(Path::new("/usr/local/bin/nemonik")).unwrap();
        assert_eq!(
            cmd,
            "run-shell \"'/usr/local/bin/nemonik' cleanup-skills #{q:hook_session_name}\""
        );
        let subcommand = cmd.split_whitespace().nth(2).unwrap();
        match crate::parse(&[subcommand, "sess"]) {
            crate::Command::CleanupSkills { session } => assert_eq!(session, "sess"),
            _ => panic!("the hook must run `cleanup-skills`"),
        }
    }

    #[test]
    fn session_closed_hook_command_is_none_when_the_exe_path_contains_a_single_quote() {
        assert!(session_closed_hook_command(Path::new("/weird/it's here/nemonik")).is_none());
    }
}
