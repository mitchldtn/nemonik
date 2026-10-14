//! The `run` pipeline: fetch, trust gate, plan, render, launch in a tmux session.

use anyhow::{Context, Result};
use clap::ValueEnum;
use nemonik_core::mcp::{HookSpec, KitBundle, McpKind, McpMode, McpSpec, SkillSpec};
use nemonik_core::skills::SkillLayout;
use std::collections::BTreeMap;
use std::io::IsTerminal;

fn agent_skill_layout(agent: &Agent) -> SkillLayout {
    match agent {
        Agent::Claude => SkillLayout::Claude,
        Agent::Codex => SkillLayout::Codex,
    }
}

#[derive(Clone, Debug, PartialEq, ValueEnum)]
pub enum Agent {
    Claude,
    Codex,
}

pub struct RunFlags {
    /// Don't attach to the session; print how to, and return.
    pub no_attach: bool,
    /// Open the session in a new terminal window via `new_window_command` in `~/.nemonik/config.json`.
    pub new: bool,
    /// Skip the agent's own permission/approval prompts entirely.
    pub yolo: bool,
    /// Allow remote control of the session. Requires `project`.
    pub remote: bool,
    /// Launch an unpinned or changed kit without reviewing or pinning it.
    pub yes: bool,
}

fn agent_binary(agent: &Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude",
        Agent::Codex => "codex",
    }
}

// Unique per call (pid + nanos): `--remote` addresses sessions by this name across
// machines, so a collision could send commands to the wrong terminal.
fn session_name(project: Option<&str>, template: &str) -> Result<String> {
    let project = project.unwrap_or("_");
    let sanitized = format!("nemonik-{project}-{template}").replace([':', '/'], "-");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    Ok(format!("{sanitized}-{:x}{nanos:x}", std::process::id()))
}

/// TOML basic-string escaping for Codex `-c` values (Rust's `Debug` escaping isn't valid TOML).
fn toml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// Always-on flags. Claude's `--verbose` makes its transcript show what was injected.
fn base_args(agent: &Agent) -> Vec<String> {
    match agent {
        Agent::Claude => vec!["--verbose".to_string()],
        Agent::Codex => Vec::new(),
    }
}

/// Claude `--mcp-config` JSON. Each stdio server gets only its own `env`, so secrets
/// belong there. Errors on a stdio MCP without `command` or an http MCP without `url`.
fn build_mcp_config_json(mcps: &[McpSpec]) -> Result<String> {
    let mut servers = serde_json::Map::new();
    for mcp in mcps {
        let entry = match mcp.kind {
            McpKind::Stdio => {
                let command = mcp.command.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("MCP '{}' is type stdio but has no command", mcp.name)
                })?;
                serde_json::json!({
                    "type": "stdio",
                    "command": command,
                    "args": mcp.args,
                    "env": mcp.env,
                })
            }
            McpKind::Http => {
                let url = mcp.url.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("MCP '{}' is type http but has no url", mcp.name)
                })?;
                serde_json::json!({
                    "type": "http",
                    "url": url,
                    "headers": mcp.headers,
                })
            }
        };
        servers.insert(mcp.name.clone(), entry);
    }
    Ok(serde_json::json!({ "mcpServers": servers }).to_string())
}

/// Claude `--settings` JSON merging every hook's events (arrays concatenated, name-sorted;
/// non-array values skipped). Inline-JSON `--settings` needs Claude Code 2.1.205+, and it
/// layers over the user's own settings.
fn build_settings_json(hooks: &[nemonik_core::mcp::HookSpec]) -> Option<String> {
    if hooks.is_empty() {
        return None;
    }
    let mut sorted: Vec<&nemonik_core::mcp::HookSpec> = hooks.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut merged = serde_json::Map::new();
    for hook in sorted {
        let serde_json::Value::Object(events) = &hook.config else {
            continue;
        };
        for (event, value) in events {
            // Skip non-arrays so a malformed hook can't overwrite a valid one.
            let serde_json::Value::Array(incoming) = value else {
                continue;
            };
            match merged.get_mut(event) {
                Some(serde_json::Value::Array(existing)) => {
                    existing.extend(incoming.iter().cloned())
                }
                _ => {
                    merged.insert(event.clone(), serde_json::Value::Array(incoming.clone()));
                }
            }
        }
    }
    Some(serde_json::json!({ "hooks": serde_json::Value::Object(merged) }).to_string())
}

/// The agent-specific injection flags. Claude: `--append-system-prompt`, `--mcp-config`
/// (plus `--strict-mcp-config` only for `Only`, which drops the agent's own MCPs), and
/// `--settings` for hooks. Codex: `-c developer_instructions` and `-c mcp_servers.*`.
/// An empty prompt is omitted, since an empty value would still set the field.
fn build_args(plan: &LaunchPlan) -> Result<Vec<String>> {
    match plan.agent {
        Agent::Claude => {
            let mut args = Vec::new();
            if let Some(content) = &plan.system_prompt {
                args.push("--append-system-prompt".to_string());
                args.push(content.clone());
            }
            if !plan.mcp_servers.is_empty() {
                args.push(format!(
                    "--mcp-config={}",
                    build_mcp_config_json(&plan.mcp_servers)?
                ));
                if plan.mcp_mode == McpMode::Only {
                    args.push("--strict-mcp-config".to_string());
                }
            }
            if let Some(settings) = build_settings_json(&plan.hooks) {
                args.push("--settings".to_string());
                args.push(settings);
            }
            Ok(args)
        }
        Agent::Codex => {
            let mut args = Vec::new();
            if let Some(content) = &plan.system_prompt {
                args.push("-c".to_string());
                args.push(format!(
                    "developer_instructions=\"{}\"",
                    toml_escape(content)
                ));
            }
            // Codex has no `--strict-mcp-config`; these always extend (`plan` warns).
            args.extend(codex_mcp_args(&plan.mcp_servers)?);
            Ok(args)
        }
    }
}

/// A TOML bare key (`A-Z a-z 0-9 _ -`), the only segment Codex's `-c` path parser accepts.
fn is_toml_bare_key(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Codex `-c mcp_servers.<name>.*` overrides (the shape `codex mcp add` writes). Errors
/// on a stdio MCP without `command` or an http MCP without `url`.
fn codex_mcp_args(mcps: &[McpSpec]) -> Result<Vec<String>> {
    let mut args = Vec::new();
    for mcp in mcps {
        // The name becomes a dotted-key segment: a `.` would nest tables and quoted
        // segments are rejected, so fail closed on a non-bare-key name.
        if !is_toml_bare_key(&mcp.name) {
            anyhow::bail!(
                "MCP '{}' has a name that isn't a TOML bare key (letters, digits, '_' or '-' only), \
                 which --agent codex can't express as a `-c mcp_servers.<name>.*` override. \
                 Rename the MCP, or launch it with --agent claude.",
                mcp.name
            );
        }
        let ns = format!("mcp_servers.{}", mcp.name);
        match mcp.kind {
            McpKind::Stdio => {
                let command = mcp.command.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("MCP '{}' is type stdio but has no command", mcp.name)
                })?;
                args.push("-c".to_string());
                args.push(format!("{ns}.command=\"{}\"", toml_escape(command)));
                if !mcp.args.is_empty() {
                    let list = mcp
                        .args
                        .iter()
                        .map(|a| format!("\"{}\"", toml_escape(a)))
                        .collect::<Vec<_>>()
                        .join(",");
                    args.push("-c".to_string());
                    args.push(format!("{ns}.args=[{list}]"));
                }
                for (key, value) in &mcp.env {
                    // Same TOML-bare-key constraint on the env key segment.
                    if !is_toml_bare_key(key) {
                        anyhow::bail!(
                            "MCP '{}' has an env var '{key}' whose name isn't a TOML bare key (letters, \
                             digits, '_' or '-' only), which --agent codex can't express as a \
                             `-c mcp_servers.<name>.env.<key>` override. Launch it with --agent claude.",
                            mcp.name
                        );
                    }
                    args.push("-c".to_string());
                    args.push(format!("{ns}.env.{key}=\"{}\"", toml_escape(value)));
                }
            }
            McpKind::Http => {
                let url = mcp.url.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("MCP '{}' is type http but has no url", mcp.name)
                })?;
                args.push("-c".to_string());
                args.push(format!("{ns}.url=\"{}\"", toml_escape(url)));
            }
        }
    }
    Ok(args)
}

/// The first user turn: the workflow (when injected into it) before the task,
/// separated by a blank line.
fn first_turn(workflow: Option<&str>, prompt: Option<&str>) -> Option<String> {
    match (workflow, prompt) {
        (Some(workflow), Some(prompt)) => Some(format!("{workflow}\n\n{prompt}")),
        (Some(workflow), None) => Some(workflow.to_string()),
        (None, Some(prompt)) => Some(prompt.to_string()),
        (None, None) => None,
    }
}

/// `--yolo`: skips the agent's own permission prompts. Independent of the trust gate.
fn yolo_args(agent: &Agent) -> Vec<String> {
    match agent {
        Agent::Claude => vec!["--dangerously-skip-permissions".to_string()],
        Agent::Codex => vec!["--dangerously-bypass-approvals-and-sandbox".to_string()],
    }
}

/// A fully-resolved, data-only launch: `plan` builds it, `render` turns it into argv,
/// and `run` does the effects. Skills ride along as data; materializing them is an effect.
#[derive(Debug, PartialEq)]
struct LaunchPlan {
    agent: Agent,
    /// `None` for an absent or empty prompt, so no injection flag is emitted.
    system_prompt: Option<String>,
    mcp_servers: Vec<McpSpec>,
    /// Only consulted when `mcp_servers` is non-empty.
    mcp_mode: McpMode,
    /// Dropped for Codex, with a warning.
    hooks: Vec<HookSpec>,
    skills: Vec<SkillSpec>,
    /// Appended verbatim, in order, after the injection flags.
    launch_args: Vec<String>,
    /// The first user turn (see `first_turn`). `None` when there is neither a
    /// user-injected workflow nor a task prompt.
    initial_prompt: Option<String>,
    /// Skip the agent's own permission prompts (`yolo_args`).
    yolo: bool,
    /// Advisories printed before launch (Codex only, today).
    warnings: Vec<String>,
}

/// What a Codex launch can't fully honor; warnings, not refusals.
fn codex_launch_warnings(agent: &Agent, bundle: &KitBundle) -> Vec<String> {
    if *agent != Agent::Codex {
        return Vec::new();
    }
    let mut warnings = Vec::new();
    if !bundle.hooks.is_empty() {
        warnings.push(format!(
            "{} hook(s) attached: hooks are not yet applied on --agent codex; launching without them.",
            bundle.hooks.len()
        ));
    }
    if bundle.mcp_mode == McpMode::Only && !bundle.mcps.is_empty() {
        warnings.push(
            "mcp_mode=only can't be enforced on --agent codex (no --strict-mcp-config equivalent); the kit's \
             MCP servers extend Codex's configured servers rather than replacing them."
                .to_string(),
        );
    }
    let http_lossy = bundle
        .mcps
        .iter()
        .filter(|m| m.kind == McpKind::Http && !(m.headers.is_empty() && m.env.is_empty()))
        .count();
    if http_lossy > 0 {
        warnings.push(format!(
            "{http_lossy} http MCP(s) carry custom headers or env, which --agent codex can't convey \
             (it receives only the server url); launching those servers without them."
        ));
    }
    warnings
}

/// Resolves the bundle the trust gate let through into a plan.
fn plan(agent: &Agent, prompt: Option<&str>, yolo: bool, bundle: &KitBundle) -> LaunchPlan {
    // A present, non-empty workflow goes to the system prompt, the first turn, or both.
    use nemonik_core::mcp::WorkflowInject;
    let workflow = bundle.workflow.as_deref().filter(|s| !s.is_empty());
    let inject_system = workflow.is_some()
        && matches!(
            bundle.workflow_inject,
            WorkflowInject::System | WorkflowInject::Both
        );
    let inject_user = workflow.is_some()
        && matches!(
            bundle.workflow_inject,
            WorkflowInject::User | WorkflowInject::Both
        );

    // `None` for an absent or empty prompt; the workflow is appended when injected there.
    let base_prompt = bundle
        .prompt
        .as_ref()
        .map(|p| p.content.clone())
        .filter(|c| !c.is_empty());
    let system_prompt = match (base_prompt, if inject_system { workflow } else { None }) {
        (Some(p), Some(w)) => Some(format!("{p}\n\n{w}")),
        (Some(p), None) => Some(p),
        (None, Some(w)) => Some(w.to_string()),
        (None, None) => None,
    };

    let initial_prompt = first_turn(if inject_user { workflow } else { None }, prompt);

    LaunchPlan {
        agent: agent.clone(),
        system_prompt,
        mcp_servers: bundle.mcps.clone(),
        mcp_mode: bundle.mcp_mode,
        hooks: bundle.hooks.clone(),
        skills: bundle.skills.clone(),
        launch_args: bundle.launch_args.clone(),
        initial_prompt,
        yolo,
        warnings: codex_launch_warnings(agent, bundle),
    }
}

/// The options: base args, injection flags, launch args, yolo. `finish_argv` adds the
/// first turn.
fn render(plan: &LaunchPlan) -> Result<Vec<String>> {
    let mut args = base_args(&plan.agent);
    args.extend(build_args(plan)?);
    // Launch args are gated by the trust review, not here.
    args.extend(plan.launch_args.iter().cloned());
    if plan.yolo {
        args.extend(yolo_args(&plan.agent));
    }
    Ok(args)
}

/// `--remote` needs a real project (the server requires one); checked before any fetch.
fn validate_remote(remote: bool, project: Option<&str>) -> Result<()> {
    if remote && project.is_none() {
        anyhow::bail!("--remote requires --project");
    }
    Ok(())
}

pub async fn run(
    project: Option<&str>,
    template: &str,
    agent: Agent,
    flags: RunFlags,
    prompt: Option<&str>,
    trust_flag: Option<&str>,
) -> Result<()> {
    let RunFlags {
        no_attach,
        new,
        yolo,
        remote,
        yes,
    } = flags;
    validate_remote(remote, project)?;
    // Fail before creating a session if `--new` has no configured command.
    if new {
        nemonik_core::config::require_new_window_command()?;
    }

    let bundle = crate::assemble::resolve_bundle(template).await?;

    let verified = crate::trust::bundle_is_verified(template, &bundle)?;
    let policy = nemonik_core::config::resolve_trust(trust_flag)?;
    let interactive = std::io::stdin().is_terminal();
    let review = || crate::trust::review_inline(template, &bundle);
    match crate::trust::gate(policy, verified, interactive, yes, template, review)? {
        crate::trust::Gate::Launch { pin, notice } => {
            if let Some(notice) = notice {
                eprintln!("{notice}");
            }
            if pin {
                crate::trust::pin(template, &bundle)?;
            }
        }
        crate::trust::Gate::Refuse(reason) => anyhow::bail!(reason),
    }

    let launch = plan(&agent, prompt, yolo, &bundle);
    for warning in &launch.warnings {
        eprintln!("nemonik: {warning}");
    }
    // One session name for both the skills base and the spec.
    let session = session_name(project, template)?;
    let (args, env) = finish_argv(&launch, &session)?;
    let spec = nemonik_core::launchspec::LaunchSpec {
        program: agent_binary(&launch.agent).to_string(),
        args,
        session,
        template: template.to_string(),
        skills: launch.skills.clone(),
        env,
        skills_layout: agent_skill_layout(&launch.agent),
    };
    // `validate_remote` guarantees a project when `--remote`.
    let remote_project = if remote { project } else { None };
    crate::session::launch(&spec, no_attach, new, remote_project)
}

/// The agent's argv and env for `plan`: the options, this session's skills (Claude
/// `--add-dir <base>`, Codex `CODEX_HOME`), then `--` before the prompt, so a
/// multi-value option at the end of `launch_args` can't take it as a value.
fn finish_argv(
    plan: &LaunchPlan,
    session: &str,
) -> Result<(Vec<String>, BTreeMap<String, String>)> {
    let mut args = render(plan)?;
    let mut env = BTreeMap::new();
    if !plan.skills.is_empty() {
        match agent_skill_layout(&plan.agent) {
            SkillLayout::Claude => args = with_skill_dir_arg(args, session)?,
            SkillLayout::Codex => {
                let home = nemonik_core::skills::codex_home_dir(session)?;
                // Fail closed on a non-UTF-8 path rather than point Codex at the wrong home.
                let home = home.to_str().ok_or_else(|| {
                    anyhow::anyhow!("codex home path is not valid UTF-8: {}", home.display())
                })?;
                env.insert("CODEX_HOME".to_string(), home.to_string());
            }
        }
    }
    // Last, as `claude [options] -- [prompt]` and `codex [options] -- [prompt]`.
    if let Some(prompt) = &plan.initial_prompt {
        args.push("--".to_string());
        args.push(prompt.clone());
    }
    Ok((args, env))
}

/// Appends `--add-dir <session base>`. It is variadic, so `finish_argv` puts `--` after
/// it. Claude scanning added dirs for skills is observed behavior (2.1.205), not a
/// documented contract.
fn with_skill_dir_arg(mut args: Vec<String>, session: &str) -> Result<Vec<String>> {
    let base = nemonik_core::skills::session_base(session)?;
    // Fail closed on a non-UTF-8 base: a lossy path would point Claude elsewhere.
    let base = base.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "session skills base path is not valid UTF-8: {}",
            base.display()
        )
    })?;
    args.push("--add-dir".to_string());
    args.push(base.to_string());
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_remote_requires_project() {
        assert!(validate_remote(true, None).is_err());
    }

    #[test]
    fn session_name_is_unique_per_call() {
        assert_ne!(
            session_name(Some("proj"), "tmpl").unwrap(),
            session_name(Some("proj"), "tmpl").unwrap()
        );
    }

    fn bundle_with_content(content: &str) -> KitBundle {
        KitBundle {
            id: None,
            prompt: Some(nemonik_core::mcp::PromptSpec {
                content: content.to_string(),
            }),
            skills: vec![],
            mcps: vec![],
            mcp_mode: nemonik_core::mcp::McpMode::Extend,
            hooks: vec![],
            launch_args: vec![],
            workflow: None,
            workflow_inject: Default::default(),
        }
    }

    fn sample_mcp(name: &str) -> McpSpec {
        McpSpec {
            name: name.to_string(),
            kind: McpKind::Stdio,
            command: Some("sh".to_string()),
            args: vec!["-c".to_string(), "echo $TEST_KEY".to_string()],
            url: None,
            headers: Default::default(),
            env: std::collections::BTreeMap::from([("TEST_KEY".to_string(), "secret".to_string())]),
        }
    }

    fn sample_skill(name: &str) -> nemonik_core::mcp::SkillSpec {
        nemonik_core::mcp::SkillSpec {
            name: name.to_string(),
            description: "d".to_string(),
            body: "b".to_string(),
        }
    }

    // A plan for a pinned bundle, as `run` builds it: no `--yolo`, no initial prompt.
    fn planned(agent: &Agent, bundle: &KitBundle) -> LaunchPlan {
        plan(agent, None, false, bundle)
    }

    #[test]
    fn first_turn_puts_the_workflow_first_with_a_blank_line() {
        assert_eq!(
            first_turn(Some("WF"), Some("review PR 1")),
            Some("WF\n\nreview PR 1".to_string())
        );
    }

    #[test]
    fn build_args_for_claude_appends_system_prompt_flag() {
        let args = build_args(&planned(&Agent::Claude, &bundle_with_content("hello"))).unwrap();
        assert_eq!(
            args,
            vec!["--append-system-prompt".to_string(), "hello".to_string()]
        );
    }

    #[test]
    fn build_args_for_codex_escapes_toml_special_characters() {
        let args = build_args(&planned(
            &Agent::Codex,
            &bundle_with_content("line one\nline \"two\""),
        ))
        .unwrap();
        assert_eq!(args[0], "-c");
        assert_eq!(
            args[1],
            "developer_instructions=\"line one\\nline \\\"two\\\"\""
        );
    }

    #[test]
    fn build_args_omits_injection_flags_for_a_prompt_less_kit_on_claude() {
        let empty = KitBundle {
            id: None,
            prompt: None,
            skills: vec![],
            mcps: vec![],
            mcp_mode: nemonik_core::mcp::McpMode::Extend,
            hooks: vec![],
            launch_args: vec![],
            workflow: None,
            workflow_inject: Default::default(),
        };
        assert_eq!(
            build_args(&planned(&Agent::Claude, &empty)).unwrap(),
            Vec::<String>::new()
        );
    }

    fn hook(name: &str, event: &str, command: &str) -> nemonik_core::mcp::HookSpec {
        nemonik_core::mcp::HookSpec {
            name: name.to_string(),
            description: None,
            config: serde_json::json!({
                event: [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": command }] }]
            }),
        }
    }

    #[test]
    fn build_settings_json_concats_same_event_and_keeps_distinct_events() {
        let hooks = vec![
            hook("a", "PreToolUse", "echo A"),
            hook("b", "PreToolUse", "echo B"),
            hook("c", "SessionStart", "echo S"),
        ];
        let json = build_settings_json(&hooks).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let pre = v["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0]["hooks"][0]["command"], serde_json::json!("echo A"));
        assert_eq!(pre[1]["hooks"][0]["command"], serde_json::json!("echo B"));
        assert_eq!(v["hooks"]["SessionStart"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn build_settings_json_skips_a_non_array_event_value_without_clobbering() {
        // The malformed hook sorts first, so the valid array survives regardless of order.
        let malformed = nemonik_core::mcp::HookSpec {
            name: "a".to_string(),
            description: None,
            config: serde_json::json!({ "PreToolUse": "oops-not-an-array" }),
        };
        let valid = hook("b", "PreToolUse", "echo A");
        let json = build_settings_json(&[malformed, valid]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let pre = v["hooks"]["PreToolUse"]
            .as_array()
            .expect("PreToolUse stays an array");
        assert_eq!(pre.len(), 1);
        assert_eq!(pre[0]["hooks"][0]["command"], serde_json::json!("echo A"));
    }

    #[test]
    fn build_settings_json_is_deterministic_regardless_of_hook_order() {
        let forward = vec![
            hook("a", "PreToolUse", "echo A"),
            hook("b", "PreToolUse", "echo B"),
        ];
        let reversed = vec![
            hook("b", "PreToolUse", "echo B"),
            hook("a", "PreToolUse", "echo A"),
        ];
        assert_eq!(
            build_settings_json(&forward),
            build_settings_json(&reversed)
        );
    }

    #[test]
    fn build_args_adds_settings_for_a_kit_with_hooks_on_claude() {
        let mut bundle = bundle_with_content("p");
        bundle.hooks = vec![hook("a", "PreToolUse", "echo A")];
        let args = build_args(&planned(&Agent::Claude, &bundle)).unwrap();
        let idx = args
            .iter()
            .position(|a| a == "--settings")
            .expect("--settings present");
        let settings = &args[idx + 1];
        assert!(settings.contains("\"PreToolUse\""));
        assert!(settings.contains("echo A"));
    }

    #[test]
    fn build_args_for_codex_drops_hooks_without_erroring() {
        let mut bundle = bundle_with_content("p");
        bundle.hooks = vec![hook("a", "PreToolUse", "echo A")];
        let args = build_args(&planned(&Agent::Codex, &bundle)).unwrap();
        assert_eq!(
            args,
            vec!["-c".to_string(), "developer_instructions=\"p\"".to_string()]
        );
    }

    #[test]
    fn build_args_adds_mcp_config_without_strict_flag_for_claude_in_extend_mode() {
        let mut bundle = bundle_with_content("hello");
        bundle.mcps = vec![sample_mcp("format")];
        let args = build_args(&planned(&Agent::Claude, &bundle)).unwrap();

        assert_eq!(
            args,
            vec![
                "--append-system-prompt".to_string(),
                "hello".to_string(),
                format!(
                    "--mcp-config={}",
                    build_mcp_config_json(&bundle.mcps).unwrap()
                ),
            ]
        );
    }

    #[test]
    fn build_args_adds_strict_flag_for_claude_in_only_mode() {
        let mut bundle = bundle_with_content("hello");
        bundle.mcps = vec![sample_mcp("format")];
        bundle.mcp_mode = nemonik_core::mcp::McpMode::Only;
        let args = build_args(&planned(&Agent::Claude, &bundle)).unwrap();

        assert_eq!(
            args,
            vec![
                "--append-system-prompt".to_string(),
                "hello".to_string(),
                format!(
                    "--mcp-config={}",
                    build_mcp_config_json(&bundle.mcps).unwrap()
                ),
                "--strict-mcp-config".to_string(),
            ]
        );
    }

    #[test]
    fn build_args_for_codex_renders_mcps_as_config_overrides() {
        let mut bundle = bundle_with_content("hello");
        bundle.mcps = vec![sample_mcp("format")];
        let args = build_args(&planned(&Agent::Codex, &bundle)).unwrap();
        assert_eq!(
            args,
            vec![
                "-c".to_string(),
                "developer_instructions=\"hello\"".to_string(),
                "-c".to_string(),
                "mcp_servers.format.command=\"sh\"".to_string(),
                "-c".to_string(),
                "mcp_servers.format.args=[\"-c\",\"echo $TEST_KEY\"]".to_string(),
                "-c".to_string(),
                "mcp_servers.format.env.TEST_KEY=\"secret\"".to_string(),
            ]
        );
    }

    #[test]
    fn codex_mcp_args_renders_an_http_server_as_a_url_override() {
        let http = McpSpec {
            name: "docs".to_string(),
            kind: McpKind::Http,
            command: None,
            args: vec![],
            url: Some("https://example.com/mcp".to_string()),
            headers: Default::default(),
            env: Default::default(),
        };
        assert_eq!(
            codex_mcp_args(&[http]).unwrap(),
            vec![
                "-c".to_string(),
                "mcp_servers.docs.url=\"https://example.com/mcp\"".to_string()
            ]
        );
    }

    #[test]
    fn codex_mcp_args_fails_closed_on_a_name_that_isnt_a_toml_bare_key() {
        let bad = McpSpec {
            name: "git.hub".to_string(),
            kind: McpKind::Http,
            command: None,
            args: vec![],
            url: Some("https://x/mcp".to_string()),
            headers: Default::default(),
            env: Default::default(),
        };
        let err = codex_mcp_args(&[bad]).unwrap_err().to_string();
        assert!(err.contains("TOML bare key"), "got: {err}");
    }

    #[test]
    fn codex_mcp_args_fails_closed_on_an_env_key_that_isnt_a_toml_bare_key() {
        let bad = McpSpec {
            name: "ok".to_string(),
            kind: McpKind::Stdio,
            command: Some("run".to_string()),
            args: vec![],
            url: None,
            headers: Default::default(),
            env: std::collections::BTreeMap::from([("BAD.KEY".to_string(), "v".to_string())]),
        };
        assert!(codex_mcp_args(&[bad]).is_err());
    }

    #[test]
    fn is_toml_bare_key_accepts_slugs_and_rejects_dotted_or_empty() {
        assert!(is_toml_bare_key("aws-cli"));
        assert!(is_toml_bare_key("TEST_KEY"));
        assert!(!is_toml_bare_key("git.hub"));
        assert!(!is_toml_bare_key("has space"));
        assert!(!is_toml_bare_key(""));
    }

    #[test]
    fn codex_launch_warnings_flags_dropped_hooks_and_only_mode_and_headers() {
        let mut bundle = bundle_with_content("p");
        bundle.hooks = vec![hook("a", "PreToolUse", "echo A")];
        bundle.mcps = vec![McpSpec {
            name: "docs".to_string(),
            kind: McpKind::Http,
            command: None,
            args: vec![],
            url: Some("https://example.com/mcp".to_string()),
            headers: std::collections::BTreeMap::from([(
                "Authorization".to_string(),
                "Bearer x".to_string(),
            )]),
            env: Default::default(),
        }];
        bundle.mcp_mode = McpMode::Only;
        let w = codex_launch_warnings(&Agent::Codex, &bundle);
        assert_eq!(w.len(), 3, "hooks + only-mode + headers: {w:?}");
        assert!(w.iter().any(|m| m.contains("hook(s)")));
        assert!(w.iter().any(|m| m.contains("mcp_mode=only")));
        assert!(w.iter().any(|m| m.contains("custom headers")));
    }

    #[test]
    fn plan_workflow_both_injects_system_and_user() {
        let mut bundle = bundle_with_content("P");
        bundle.skills = vec![sample_skill("gh")];
        bundle.workflow = Some("WF".to_string());
        bundle.workflow_inject = nemonik_core::mcp::WorkflowInject::Both;
        let p = plan(&Agent::Claude, Some("do X"), false, &bundle);
        assert_eq!(p.system_prompt.as_deref(), Some("P\n\nWF"));
        assert_eq!(p.initial_prompt.as_deref(), Some("WF\n\ndo X"));
    }

    #[test]
    fn plan_workflow_system_injects_only_the_system_prompt() {
        // system -> system prompt only; first turn is just the task.
        let mut bundle = bundle_with_content("P");
        bundle.skills = vec![sample_skill("gh")];
        bundle.workflow = Some("WF".to_string());
        bundle.workflow_inject = nemonik_core::mcp::WorkflowInject::System;
        let p = plan(&Agent::Claude, Some("do X"), false, &bundle);
        assert_eq!(p.system_prompt.as_deref(), Some("P\n\nWF"));
        assert_eq!(p.initial_prompt.as_deref(), Some("do X"));
    }

    #[test]
    fn plan_workflow_user_injects_only_the_first_turn() {
        // user -> first turn only; system prompt stays the kit prompt.
        let mut bundle = bundle_with_content("P");
        bundle.skills = vec![sample_skill("gh")];
        bundle.workflow = Some("WF".to_string());
        bundle.workflow_inject = nemonik_core::mcp::WorkflowInject::User;
        let p = plan(&Agent::Claude, Some("do X"), false, &bundle);
        assert_eq!(p.system_prompt.as_deref(), Some("P"));
        assert_eq!(p.initial_prompt.as_deref(), Some("WF\n\ndo X"));
    }

    #[test]
    fn plan_empty_workflow_is_treated_as_absent() {
        // Matches bundle_hash's skip, so a ""-workflow kit's verify pin is unaffected.
        let mut bundle = bundle_with_content("P");
        bundle.skills = vec![sample_skill("gh")];
        bundle.workflow = Some(String::new());
        bundle.workflow_inject = nemonik_core::mcp::WorkflowInject::Both;
        let p = plan(&Agent::Claude, Some("do X"), false, &bundle);
        assert_eq!(p.system_prompt.as_deref(), Some("P"));
        assert_eq!(p.initial_prompt.as_deref(), Some("do X"));
    }

    #[test]
    fn build_mcp_config_json_renders_claudes_stdio_shape() {
        let json = build_mcp_config_json(&[sample_mcp("format")]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "mcpServers": {
                    "format": {
                        "type": "stdio",
                        "command": "sh",
                        "args": ["-c", "echo $TEST_KEY"],
                        "env": {"TEST_KEY": "secret"},
                    }
                }
            })
        );
    }

    #[test]
    fn build_mcp_config_json_renders_claudes_http_shape() {
        let mcp = McpSpec {
            name: "webhook".to_string(),
            kind: McpKind::Http,
            command: None,
            args: vec![],
            url: Some("https://example.com/hook".to_string()),
            headers: std::collections::BTreeMap::from([("X-Test".to_string(), "1".to_string())]),
            env: Default::default(),
        };
        let json = build_mcp_config_json(&[mcp]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "mcpServers": {
                    "webhook": {
                        "type": "http",
                        "url": "https://example.com/hook",
                        "headers": {"X-Test": "1"},
                    }
                }
            })
        );
    }

    #[test]
    fn build_mcp_config_json_errors_clearly_for_a_stdio_mcp_missing_a_command() {
        let mcp = McpSpec {
            name: "broken".to_string(),
            kind: McpKind::Stdio,
            command: None,
            args: vec![],
            url: None,
            headers: Default::default(),
            env: Default::default(),
        };
        let err = build_mcp_config_json(&[mcp]).unwrap_err();
        assert!(format!("{err}").contains("broken"));
        assert!(format!("{err}").contains("command"));
    }

    #[test]
    fn build_mcp_config_json_errors_clearly_for_an_http_mcp_missing_a_url() {
        let mcp = McpSpec {
            name: "broken".to_string(),
            kind: McpKind::Http,
            command: None,
            args: vec![],
            url: None,
            headers: Default::default(),
            env: Default::default(),
        };
        let err = build_mcp_config_json(&[mcp]).unwrap_err();
        assert!(format!("{err}").contains("broken"));
        assert!(format!("{err}").contains("url"));
    }

    #[test]
    fn toml_escape_handles_control_characters_as_uxxxx_not_rust_debug_braces() {
        let escaped = toml_escape("\u{01}");
        assert_eq!(escaped, "\\u0001");
    }

    #[test]
    fn render_orders_base_then_injection_then_yolo() {
        let bundle = bundle_with_content("hello");
        let p = plan(&Agent::Claude, None, true, &bundle);
        assert_eq!(
            render(&p).unwrap(),
            vec![
                "--verbose".to_string(),
                "--append-system-prompt".to_string(),
                "hello".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ]
        );
    }

    #[test]
    fn render_injects_launch_args_after_the_injection_flags() {
        let mut bundle = bundle_with_content("sys");
        bundle.launch_args = vec!["--model".into(), "fable".into()];
        let p = plan(&Agent::Claude, Some("review PR 7"), false, &bundle);
        assert_eq!(
            render(&p).unwrap(),
            vec![
                "--verbose".to_string(),
                "--append-system-prompt".to_string(),
                "sys".to_string(),
                "--model".to_string(),
                "fable".to_string(),
            ]
        );
    }

    #[test]
    fn render_mcp_config_is_attached_so_it_cannot_swallow_the_trailing_prompt() {
        // `=` form keeps the trailing prompt from being swallowed by variadic `--mcp-config`.
        let mut bundle = bundle_with_content("sys");
        bundle.mcps = vec![sample_mcp("format")];
        let p = plan(
            &Agent::Claude,
            Some("what is my buying power"),
            false,
            &bundle,
        );
        let args = render(&p).unwrap();
        assert!(args.iter().any(|a| a.starts_with("--mcp-config={")));
        assert!(!args.iter().any(|a| a == "--mcp-config"));
    }

    #[test]
    fn render_carries_hooks_through_to_a_single_settings_flag() {
        let mut bundle = bundle_with_content("sys");
        bundle.hooks = vec![hook("a", "PreToolUse", "echo A")];
        let p = plan(&Agent::Claude, None, false, &bundle);
        let args = render(&p).unwrap();
        let idx = args
            .iter()
            .position(|a| a == "--settings")
            .expect("--settings present");
        assert!(args[idx + 1].contains("echo A"));
    }

    #[test]
    fn finish_argv_claude_ends_options_after_the_skill_dir() {
        // `--allowedTools` takes several values; without `--` it would take the prompt.
        let mut bundle = bundle_with_content("sys");
        bundle.launch_args = vec!["--allowedTools".into(), "Bash".into()];
        bundle.skills = vec![sample_skill("gh")];
        let session = "nemonik-proj-tmpl-abcd";
        let base_dir = nemonik_core::skills::session_base(session).unwrap();
        let base_dir = base_dir.to_str().unwrap();

        let p = plan(&Agent::Claude, Some("do X"), false, &bundle);
        let (args, env) = finish_argv(&p, session).unwrap();
        assert!(
            args.ends_with(&[
                "--allowedTools".into(),
                "Bash".into(),
                "--add-dir".into(),
                base_dir.into(),
                "--".into(),
                "do X".into()
            ]),
            "{args:?}"
        );
        assert!(env.is_empty(), "Claude uses --add-dir, never CODEX_HOME");

        let no_prompt = plan(&Agent::Claude, None, false, &bundle);
        let (args, _) = finish_argv(&no_prompt, session).unwrap();
        assert!(
            args.ends_with(&["Bash".into(), "--add-dir".into(), base_dir.into()]),
            "{args:?}"
        );
    }

    #[test]
    fn finish_argv_codex_ends_options_and_sets_codex_home() {
        let mut bundle = bundle_with_content("sys");
        bundle.launch_args = vec!["-i".into(), "./dot.png".into()];
        bundle.skills = vec![sample_skill("gh")];
        let session = "nemonik-proj-tmpl-abcd";
        let p = plan(&Agent::Codex, Some("do X"), false, &bundle);
        let (args, env) = finish_argv(&p, session).unwrap();
        assert!(
            args.ends_with(&["./dot.png".into(), "--".into(), "do X".into()]),
            "{args:?}"
        );
        assert_eq!(
            env.get("CODEX_HOME").map(String::as_str),
            Some(
                nemonik_core::skills::codex_home_dir(session)
                    .unwrap()
                    .to_str()
                    .unwrap()
            )
        );
    }
}
