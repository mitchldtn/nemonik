//! The trust gate: a kit reaches the agent only if its fully-resolved bundle hashes to
//! a pin a human approved. Anything unpinned or changed is reviewed again or refused,
//! unless the user opts out (`nemonik trust off`, `--trust off`, or `--yes`).

use anyhow::{Context, Result};
use crossterm::style::Stylize;
use nemonik_core::config::TrustMode;
use nemonik_core::mcp::KitBundle;
use nemonik_core::review::{
    display_safe, render_args_text, render_hook_text, render_mcp_text, render_prompt_text,
    render_skill_text,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use similar::{ChangeTag, TextDiff};
use std::fmt::Write as _;
use std::io::{IsTerminal, Write as _};
use std::path::PathBuf;

/// One pin, keyed by the owner-qualified address. Stores the redacted bundle (for the
/// next diff) and the resolved id (two kits can hash identically).
#[derive(Serialize, Deserialize)]
struct VerifiedEntry {
    template: String,
    hash: String,
    bundle: KitBundle,
    resolved_id: Option<String>,
}

fn is_yes(line: &str) -> bool {
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Reads one line from stdin; end of input declines.
fn read_yes() -> Result<bool> {
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line)? == 0 {
        println!();
    }
    Ok(is_yes(&line))
}

fn verified_store_path() -> Result<PathBuf> {
    Ok(nemonik_core::auth::nemonik_dir()?.join("verified.json"))
}

fn load_verified() -> Result<Vec<VerifiedEntry>> {
    let path = verified_store_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse {}; delete it and verify your kits again",
            path.display()
        )
    })
}

fn save_verified(entries: &[VerifiedEntry]) -> Result<()> {
    let path = verified_store_path()?;
    std::fs::write(&path, serde_json::to_string_pretty(entries)?)?;
    // Secrets are already redacted (`redact_bundle_for_storage`); `0600` anyway.
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// SHA-256 over a canonical JSON of the whole bundle. Skills, MCPs, and hooks are sorted
/// by name (order isn't a change); `mcp_mode` only counts when MCPs are attached, the
/// only case where it has an effect.
fn bundle_hash(bundle: &KitBundle) -> String {
    #[derive(Serialize)]
    struct Canonical<'a> {
        prompt: Option<&'a nemonik_core::mcp::PromptSpec>,
        skills: &'a [nemonik_core::mcp::SkillSpec],
        mcps: &'a [nemonik_core::mcp::McpSpec],
        mcp_mode: nemonik_core::mcp::McpMode,
        // Hooks run with no trust prompt of their own, so they're pinned. `config`
        // hashes stably only because serde_json's `preserve_order` is off.
        hooks: &'a [nemonik_core::mcp::HookSpec],
        // NOT sorted: argument order changes what launches.
        launch_args: &'a [String],
        // An empty workflow is no workflow, and the inject mode only counts with one.
        workflow: Option<&'a str>,
        workflow_inject: Option<nemonik_core::mcp::WorkflowInject>,
    }

    let mut skills = bundle.skills.clone();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    let mut mcps = bundle.mcps.clone();
    mcps.sort_by(|a, b| a.name.cmp(&b.name));
    let mut hooks = bundle.hooks.clone();
    hooks.sort_by(|a, b| a.name.cmp(&b.name));

    let mcp_mode = if mcps.is_empty() {
        nemonik_core::mcp::McpMode::Extend
    } else {
        bundle.mcp_mode
    };
    let workflow = bundle.workflow.as_deref().filter(|s| !s.is_empty());
    let canonical = Canonical {
        prompt: bundle.prompt.as_ref(),
        skills: &skills,
        mcps: &mcps,
        mcp_mode,
        hooks: &hooks,
        launch_args: &bundle.launch_args,
        workflow,
        workflow_inject: workflow.map(|_| bundle.workflow_inject),
    };
    let json = serde_json::to_string(&canonical).expect("KitBundle always serializes to JSON");
    content_hash(&json)
}

fn hash_matches(pinned: Option<&str>, current: &KitBundle) -> bool {
    pinned == Some(bundle_hash(current).as_str())
}

/// `true` when the server sends no id (local kits have none). `false` when it does but
/// the pin has none: a local kit pinned under the same name is not trusted for it.
fn id_matches(pinned: Option<&str>, current: Option<&str>) -> bool {
    match current {
        None => true,
        Some(c) => pinned == Some(c),
    }
}

/// Both sides have an id and they differ. Lets `verify` say why a no-diff kit needs approval.
fn resolved_id_changed(pinned: Option<&str>, current: Option<&str>) -> bool {
    matches!((pinned, current), (Some(p), Some(c)) if p != c)
}

fn find_pinned<'a>(entries: &'a [VerifiedEntry], template: &str) -> Option<&'a VerifiedEntry> {
    entries.iter().find(|e| e.template == template)
}

/// The bundle as review text. With `use_color`, only the section markers this function
/// writes are colored, never kit content; diffing uses the plain (`false`) form.
fn render_bundle(bundle: &KitBundle, use_color: bool) -> String {
    let styled = |text: &str, apply: fn(&str) -> String| {
        if use_color {
            apply(text)
        } else {
            text.to_string()
        }
    };
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}\n{}",
        styled("## Prompt", |s| s.bold().blue().to_string()),
        render_prompt_text(bundle.prompt.as_ref())
    );

    if !bundle.skills.is_empty() {
        let _ = writeln!(
            out,
            "\n{}",
            styled("## Skills", |s| s.bold().magenta().to_string())
        );
        for skill in &bundle.skills {
            let _ = writeln!(
                out,
                "\n{}{}",
                styled("### ", |s| s.bold().magenta().to_string()),
                render_skill_text(skill)
            );
        }
    }

    // Red: hooks are auto-running shell.
    if !bundle.hooks.is_empty() {
        let _ = writeln!(
            out,
            "\n{}",
            styled("## Hooks", |s| s.bold().red().to_string())
        );
        for hook in &bundle.hooks {
            let _ = writeln!(
                out,
                "\n{}{}",
                styled("### ", |s| s.bold().red().to_string()),
                render_hook_text(hook)
            );
        }
    }

    if !bundle.mcps.is_empty() {
        // `only` drops the agent's own MCP servers at launch, so the mode is shown.
        let _ = writeln!(
            out,
            "\n{} (mcp mode: {:?})",
            styled("## MCPs", |s| s.bold().yellow().to_string()),
            bundle.mcp_mode
        );
        for mcp in &bundle.mcps {
            let _ = writeln!(
                out,
                "\n{}{}",
                styled("### ", |s| s.bold().yellow().to_string()),
                render_mcp_text(mcp)
            );
        }
    }

    if !bundle.launch_args.is_empty() {
        let _ = writeln!(
            out,
            "\n{}\n{}",
            styled("## Launch args", |s| s.bold().cyan().to_string()),
            render_args_text(&bundle.launch_args)
        );
    }

    // Workflow prose is injected, so it must be visible before pinning.
    if let Some(wf) = bundle.workflow.as_deref().filter(|s| !s.is_empty()) {
        let _ = writeln!(
            out,
            "\n{} (inject: {:?})",
            styled("## Workflow", |s| s.bold().green().to_string()),
            bundle.workflow_inject
        );
        let _ = writeln!(out, "{}", display_safe(wf));
    }

    out
}

/// Color only on a terminal, and only when `NO_COLOR` is unset.
fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

const REDACTED_SECRET_PLACEHOLDER: &str = "<redacted>";

/// Replaces MCP env/header values before a bundle is stored. The hash is computed over
/// the real bundle first, so a changed secret is still detected.
fn redact_bundle_for_storage(bundle: &KitBundle) -> KitBundle {
    let mcps = bundle
        .mcps
        .iter()
        .map(|mcp| {
            let mut redacted = mcp.clone();
            for value in redacted.env.values_mut() {
                *value = REDACTED_SECRET_PLACEHOLDER.to_string();
            }
            for value in redacted.headers.values_mut() {
                *value = REDACTED_SECRET_PLACEHOLDER.to_string();
            }
            redacted
        })
        .collect();
    KitBundle {
        id: bundle.id.clone(),
        prompt: bundle.prompt.clone(),
        skills: bundle.skills.clone(),
        mcps,
        mcp_mode: bundle.mcp_mode,
        // Hooks are stored verbatim (no structured secret field): a secret inlined in a
        // hook command lands in this 0600 file in plaintext.
        hooks: bundle.hooks.clone(),
        launch_args: bundle.launch_args.clone(),
        workflow: bundle.workflow.clone(),
        workflow_inject: bundle.workflow_inject,
    }
}

/// The unified diff text `print_diff` prints; each colored line resets before its newline.
fn render_diff_text(previous_rendered: &str, current_rendered: &str, use_color: bool) -> String {
    let diff = TextDiff::from_lines(previous_rendered, current_rendered);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        let text = change.to_string();
        let had_trailing_newline = text.ends_with('\n');
        let line = format!("{sign}{}", text.trim_end_matches('\n'));
        let printed = match (change.tag(), use_color) {
            (ChangeTag::Delete, true) => line.red().to_string(),
            (ChangeTag::Insert, true) => line.green().to_string(),
            _ => line,
        };
        out.push_str(&printed);
        if had_trailing_newline {
            out.push('\n');
        }
    }
    out
}

fn print_diff(previous_rendered: &str, current_rendered: &str, use_color: bool) {
    print!(
        "{}",
        render_diff_text(previous_rendered, current_rendered, use_color)
    );
}

/// Fetches the kit's fully-resolved bundle, displays it, and, on explicit
/// y/N confirmation, pins the whole bundle's hash. `web` also opens it in the browser.
pub async fn verify(template: &str, web: bool) -> Result<()> {
    let bundle = crate::assemble::resolve_bundle(template).await?;
    let use_color = color_enabled();

    // Same gate as `run` (hash and id); unchanged means nothing to review.
    if bundle_is_verified(template, &bundle)? {
        let msg = format!("Already verified (unchanged): {template}");
        println!(
            "{}",
            if use_color {
                msg.green().to_string()
            } else {
                msg
            }
        );
        return Ok(());
    }
    // Approval reads stdin, so piped input must not be able to give it.
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "'{template}' needs review, and `nemonik verify` asks for approval on a terminal. \
             Run it from an interactive shell; nothing was pinned."
        );
    }

    let rendered = render_bundle(&bundle, false);
    let styled = |text: &str, apply: fn(&str) -> String| {
        if use_color {
            apply(text)
        } else {
            text.to_string()
        }
    };

    let bottom_rule = "-----------------";
    println!("{}", framed_bundle(template, &bundle, use_color));

    let verified = load_verified()?;
    let pinned = find_pinned(&verified, template);
    let previous_rendered = pinned.map(|e| render_bundle(&e.bundle, false));
    // Compare rendered text, not structs: the stored bundle's secrets are redacted.
    if let Some(previous_rendered) = &previous_rendered
        && previous_rendered != &rendered
    {
        let diff_rule = "----- diff since last verified -----";
        println!("\n{}", styled(diff_rule, |s| s.dim().to_string()));
        print_diff(previous_rendered, &rendered, use_color);
        println!("{}", styled(bottom_rule, |s| s.dim().to_string()));
    }

    const RESOLVED_ID_NOTE: &str = "this owner-qualified address now resolves to a different kit than the one \
        previously verified, even though its content may be unchanged.";
    let id_changed = resolved_id_changed(
        pinned.and_then(|e| e.resolved_id.as_deref()),
        bundle.id.as_deref(),
    );
    if id_changed {
        println!("\nNote: {RESOLVED_ID_NOTE}");
    }

    // The terminal prompt below is the gate, not the browser view.
    if web {
        match crate::review_view::show(
            template,
            pinned.map(|e| &e.bundle),
            &bundle,
            id_changed.then_some(RESOLVED_ID_NOTE),
        ) {
            Ok(path) => println!("\nOpened {} in your browser.", path.display()),
            Err(e) => eprintln!("nemonik: couldn't write the browser view: {e:#}"),
        }
    }

    if prompt_yn(use_color)? {
        pin(template, &bundle)?;
        println!("{}", styled("Verified.", |s| s.bold().green().to_string()));
    } else {
        println!("Not verified.");
    }
    Ok(())
}

/// Hash and resolved id both match the pin. Anything else must not be injected.
pub fn bundle_is_verified(template: &str, current: &KitBundle) -> Result<bool> {
    let verified = load_verified()?;
    let pinned = find_pinned(&verified, template);
    let hash_ok = hash_matches(pinned.map(|e| e.hash.as_str()), current);
    let id_ok = id_matches(
        pinned.and_then(|e| e.resolved_id.as_deref()),
        current.id.as_deref(),
    );
    Ok(hash_ok && id_ok)
}

/// The framed bundle block shared by `verify` and `run`'s inline review.
fn framed_bundle(label: &str, bundle: &KitBundle, use_color: bool) -> String {
    let dim = |s: &str| {
        if use_color {
            s.dim().to_string()
        } else {
            s.to_string()
        }
    };
    let top = format!("----- {label} -----");
    format!(
        "{}\n{}{}",
        dim(&top),
        render_bundle(bundle, use_color),
        dim("-----------------")
    )
}

/// Pins `bundle` for `template`, replacing any prior pin. Shared by `verify` and `run`.
pub(crate) fn pin(template: &str, bundle: &KitBundle) -> Result<()> {
    let mut verified = load_verified()?;
    verified.retain(|e| e.template != template);
    verified.push(VerifiedEntry {
        template: template.to_string(),
        hash: bundle_hash(bundle),
        bundle: redact_bundle_for_storage(bundle),
        resolved_id: bundle.id.clone(),
    });
    save_verified(&verified)
}

/// `run`'s inline review for an unpinned kit on a TTY: show the bundle, ask y/N.
pub(crate) fn review_inline(template: &str, bundle: &KitBundle) -> Result<bool> {
    let use_color = color_enabled();
    println!("{}", framed_bundle(template, bundle, use_color));
    prompt_yn(use_color)
}

/// The `[y/N]` prompt shared by `verify` and `run`.
fn prompt_yn(use_color: bool) -> Result<bool> {
    let prompt_line = "Verify this content for system-prompt injection? [y/N] ";
    print!(
        "{}",
        if use_color {
            prompt_line.bold().to_string()
        } else {
            prompt_line.to_string()
        }
    );
    std::io::stdout().flush()?;
    read_yes()
}

/// What `run` does about trust for this launch.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Gate {
    /// Launch; `pin` only after an approved review. `notice` is printed first.
    Launch { pin: bool, notice: Option<String> },
    /// Refuse with this message.
    Refuse(String),
}

/// A verified kit launches under any policy, so `off` and `--yes` only affect unverified
/// kits. `review` runs only for an unverified, interactive, strict run without `--yes`.
pub(crate) fn gate(
    policy: TrustMode,
    verified: bool,
    interactive: bool,
    yes: bool,
    template: &str,
    review: impl FnOnce() -> Result<bool>,
) -> Result<Gate> {
    let launch = |pin, notice| Gate::Launch { pin, notice };
    if verified {
        return Ok(launch(false, None));
    }
    Ok(match policy {
        TrustMode::Off => launch(
            false,
            Some(format!(
                "nemonik: WARNING: trust is off; launching '{template}' WITHOUT verifying its kit."
            )),
        ),
        TrustMode::Strict if yes => launch(
            false,
            Some(format!(
                "nemonik: --yes: launching '{template}' without review (not pinned)."
            )),
        ),
        TrustMode::Strict if interactive => {
            if review()? {
                launch(true, None)
            } else {
                Gate::Refuse("Not verified. Refusing to launch.".to_string())
            }
        }
        TrustMode::Strict => Gate::Refuse(format!(
            "'{template}' is unverified and this is a non-interactive run.\n  \
             Run `nemonik verify {template}` to review and pin it, or pass \
             `--yes` to launch without pinning."
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundle(content: &str) -> KitBundle {
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

    #[test]
    fn is_yes_accepts_only_y_or_yes_in_any_case() {
        for line in ["y\n", "Y\n", "yes\n", "YES\r\n", "  y  \n", "y"] {
            assert!(is_yes(line), "{line:?}");
        }
        for line in ["", "\n", "n\n", "no\n", "ye\n", "yess\n", "y y\n", "sure\n"] {
            assert!(!is_yes(line), "{line:?}");
        }
    }

    #[test]
    fn the_gate_is_renderer_independent() {
        let bundle = sample_bundle("prompt body");
        let pinned = bundle_hash(&bundle);
        assert!(
            hash_matches(Some(&pinned), &bundle),
            "the pinned hash verifies the same bundle"
        );
        assert!(
            !hash_matches(Some(&pinned), &sample_bundle("prompt body EDITED")),
            "a content change flips the gate"
        );
    }

    #[test]
    fn id_matches_is_false_when_the_resolved_id_changed_since_verification() {
        assert!(!id_matches(Some("kit-1"), Some("kit-2")));
    }

    #[test]
    fn id_matches_is_false_when_the_pin_has_no_id_but_the_server_sends_one() {
        assert!(!id_matches(None, Some("kit-1")));
    }

    #[test]
    fn id_matches_is_true_when_the_bundle_has_no_id() {
        assert!(id_matches(Some("kit-1"), None));
        assert!(id_matches(None, None));
    }

    #[test]
    fn resolved_id_changed_is_true_when_both_sides_have_an_id_and_they_differ() {
        assert!(resolved_id_changed(Some("kit-1"), Some("kit-2")));
    }

    #[test]
    fn resolved_id_changed_is_false_when_either_side_has_no_id() {
        assert!(!resolved_id_changed(None, Some("kit-1")));
        assert!(!resolved_id_changed(Some("kit-1"), None));
        assert!(!resolved_id_changed(None, None));
    }

    #[test]
    fn bundle_hash_changes_when_an_mcps_command_changes_even_with_byte_identical_prompt() {
        let mcp = |command: &str| nemonik_core::mcp::McpSpec {
            name: "t".to_string(),
            kind: nemonik_core::mcp::McpKind::Stdio,
            command: Some(command.to_string()),
            args: vec![],
            url: None,
            headers: Default::default(),
            env: Default::default(),
        };
        let mut before = sample_bundle("identical prompt");
        before.mcps = vec![mcp("echo safe")];
        let mut after = sample_bundle("identical prompt");
        after.mcps = vec![mcp("echo danger")];

        assert_ne!(bundle_hash(&before), bundle_hash(&after));
    }

    #[test]
    fn bundle_hash_is_stable_when_mcp_mode_changes_with_no_mcps_attached() {
        let mut extend_mode = sample_bundle("p");
        extend_mode.mcp_mode = nemonik_core::mcp::McpMode::Extend;
        let mut only_mode = sample_bundle("p");
        only_mode.mcp_mode = nemonik_core::mcp::McpMode::Only;

        assert_eq!(bundle_hash(&extend_mode), bundle_hash(&only_mode));
    }

    #[test]
    fn bundle_hash_still_changes_on_mcp_mode_when_mcps_are_attached() {
        let mcp = nemonik_core::mcp::McpSpec {
            name: "t".to_string(),
            kind: nemonik_core::mcp::McpKind::Stdio,
            command: Some("echo hi".to_string()),
            args: vec![],
            url: None,
            headers: Default::default(),
            env: Default::default(),
        };
        let mut extend_mode = sample_bundle("p");
        extend_mode.mcps = vec![mcp.clone()];
        extend_mode.mcp_mode = nemonik_core::mcp::McpMode::Extend;
        let mut only_mode = sample_bundle("p");
        only_mode.mcps = vec![mcp];
        only_mode.mcp_mode = nemonik_core::mcp::McpMode::Only;

        assert_ne!(bundle_hash(&extend_mode), bundle_hash(&only_mode));
    }

    #[test]
    fn bundle_hash_is_stable_across_skill_reordering() {
        let skill_a = nemonik_core::mcp::SkillSpec {
            name: "a".into(),
            description: "d".into(),
            body: "b".into(),
        };
        let skill_b = nemonik_core::mcp::SkillSpec {
            name: "b".into(),
            description: "d".into(),
            body: "b".into(),
        };
        let mut forward = sample_bundle("p");
        forward.skills = vec![skill_a.clone(), skill_b.clone()];
        let mut reversed = sample_bundle("p");
        reversed.skills = vec![skill_b, skill_a];

        assert_eq!(bundle_hash(&forward), bundle_hash(&reversed));
    }

    fn sample_hook(name: &str, command: &str) -> nemonik_core::mcp::HookSpec {
        nemonik_core::mcp::HookSpec {
            name: name.to_string(),
            description: None,
            config: serde_json::json!({
                "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": command }] }]
            }),
        }
    }

    #[test]
    fn bundle_hash_canonical_form_is_fixed() {
        // A fixed constant: changing the canonical form invalidates every existing pin.
        let baseline = "8482b757de1ba49bcb1c197ba62b08e0468ec02fb15e95b96f50493ae62765cb";
        assert_eq!(bundle_hash(&sample_bundle("canonical form")), baseline);
    }

    #[test]
    fn bundle_hash_changes_when_only_a_hook_config_changes() {
        let mut before = sample_bundle("same prompt");
        before.hooks = vec![sample_hook("audit", "echo A")];
        let mut after = sample_bundle("same prompt");
        after.hooks = vec![sample_hook("audit", "echo B")];
        assert_ne!(bundle_hash(&before), bundle_hash(&after));
    }

    #[test]
    fn bundle_hash_changes_when_launch_args_reorder() {
        let mut forward = sample_bundle("p");
        forward.launch_args = vec!["--model".into(), "fable".into()];
        let mut reversed = sample_bundle("p");
        reversed.launch_args = vec!["fable".into(), "--model".into()];
        assert_ne!(bundle_hash(&forward), bundle_hash(&reversed));
    }

    #[test]
    fn bundle_hash_ignores_workflow_inject_and_an_empty_workflow() {
        let a = sample_bundle("p");
        let mut none_but_mode = sample_bundle("p");
        none_but_mode.workflow_inject = nemonik_core::mcp::WorkflowInject::User; // inert without a workflow
        assert_eq!(bundle_hash(&a), bundle_hash(&none_but_mode));
        let mut empty = sample_bundle("p");
        empty.workflow = Some(String::new());
        assert_eq!(bundle_hash(&a), bundle_hash(&empty));
    }

    #[test]
    fn bundle_hash_changes_when_workflow_inject_changes_with_a_workflow_present() {
        let mut both = sample_bundle("p");
        both.workflow = Some("wf".into());
        both.workflow_inject = nemonik_core::mcp::WorkflowInject::Both;
        let mut system = sample_bundle("p");
        system.workflow = Some("wf".into());
        system.workflow_inject = nemonik_core::mcp::WorkflowInject::System;
        assert_ne!(bundle_hash(&both), bundle_hash(&system));
    }

    #[test]
    fn render_bundle_shows_workflow_and_its_mode_so_a_human_reviews_it() {
        assert!(!render_bundle(&sample_bundle("p"), false).contains("Workflow"));
        let mut with_wf = sample_bundle("p");
        with_wf.workflow = Some("first invoke /nemonik".into());
        with_wf.workflow_inject = nemonik_core::mcp::WorkflowInject::System;
        let rendered = render_bundle(&with_wf, false);
        assert!(rendered.contains("## Workflow (inject: System)"));
        assert!(rendered.contains("first invoke /nemonik"));
    }

    #[test]
    fn render_bundle_masks_mcp_env_values_to_key_names_only() {
        let mcp = nemonik_core::mcp::McpSpec {
            name: "gh-mcp".to_string(),
            kind: nemonik_core::mcp::McpKind::Stdio,
            command: Some("gh".to_string()),
            args: vec![],
            url: None,
            headers: Default::default(),
            env: std::collections::BTreeMap::from([(
                "GITHUB_TOKEN".to_string(),
                "ghp_supersecret".to_string(),
            )]),
        };
        let mut bundle = sample_bundle("p");
        bundle.mcps = vec![mcp];

        let rendered = render_bundle(&bundle, false);
        assert!(rendered.contains("GITHUB_TOKEN"));
        assert!(!rendered.contains("ghp_supersecret"));
    }

    #[test]
    fn redact_bundle_for_storage_never_contains_the_real_secret_value_anywhere() {
        let mcp = nemonik_core::mcp::McpSpec {
            name: "gh-mcp".to_string(),
            kind: nemonik_core::mcp::McpKind::Stdio,
            command: Some("gh".to_string()),
            args: vec![],
            url: None,
            headers: Default::default(),
            env: std::collections::BTreeMap::from([(
                "GITHUB_TOKEN".to_string(),
                "ghp_supersecret".to_string(),
            )]),
        };
        let mut bundle = sample_bundle("p");
        bundle.mcps = vec![mcp];

        let redacted = redact_bundle_for_storage(&bundle);
        let serialized = serde_json::to_string(&redacted).unwrap();
        assert!(!serialized.contains("ghp_supersecret"));
    }

    #[test]
    fn render_bundle_output_is_identical_for_a_bundle_and_its_redacted_counterpart() {
        // Diffing compares rendered text, so a bundle and its redacted copy must
        // render identically.
        let mcp = nemonik_core::mcp::McpSpec {
            name: "gh-mcp".to_string(),
            kind: nemonik_core::mcp::McpKind::Http,
            command: None,
            args: vec![],
            url: Some("https://example.com".to_string()),
            headers: std::collections::BTreeMap::from([(
                "Authorization".to_string(),
                "Bearer real".to_string(),
            )]),
            env: std::collections::BTreeMap::from([(
                "GITHUB_TOKEN".to_string(),
                "ghp_real".to_string(),
            )]),
        };
        let mut bundle = sample_bundle("p");
        bundle.mcps = vec![mcp];

        let redacted = redact_bundle_for_storage(&bundle);
        assert_eq!(
            render_bundle(&redacted, false),
            render_bundle(&bundle, false)
        );
    }

    #[test]
    fn render_diff_text_with_color_disabled_is_plain_signed_lines() {
        let text = render_diff_text("line one\nline two\n", "line one\nline three\n", false);
        assert_eq!(text, " line one\n-line two\n+line three\n");
    }

    #[test]
    fn render_bundle_shows_launch_args_so_a_human_reviews_them() {
        let plain = sample_bundle("p");
        assert!(!render_bundle(&plain, false).contains("Launch args"));

        let mut with_args = sample_bundle("p");
        with_args.launch_args = vec!["--model".into(), "fable".into()];
        let rendered = render_bundle(&with_args, false);
        assert!(rendered.contains("## Launch args"));
        assert!(rendered.contains("1: '--model'"));
        assert!(rendered.contains("2: 'fable'"));
        assert!(!rendered.contains("--model fable"));
    }

    #[test]
    fn display_safe_output_is_not_what_gets_hashed() {
        // Pinning hashes raw content; hashing the display-safe text would never match a
        // later run.
        let content = "before\x1b[2Jafter";
        let display_safe = nemonik_core::review::display_safe;
        assert_ne!(content_hash(&display_safe(content)), content_hash(content));
    }

    #[test]
    fn gate_covers_the_full_matrix_and_reviews_only_when_it_must() {
        use TrustMode::{Off, Strict};
        let no_review = || -> Result<bool> { panic!("review must not run here") };
        let launch = |pin| Gate::Launch { pin, notice: None };

        // A verified kit launches silently under any policy, never reviewed or re-pinned.
        for policy in [Off, Strict] {
            for interactive in [false, true] {
                for yes in [false, true] {
                    let g = gate(policy, true, interactive, yes, "o/c:k", no_review).unwrap();
                    assert_eq!(g, launch(false));
                }
            }
        }
        // Unverified + off: warn and launch, never pinned.
        for interactive in [false, true] {
            for yes in [false, true] {
                let g = gate(Off, false, interactive, yes, "o/c:k", no_review).unwrap();
                assert!(matches!(g, Gate::Launch { pin: false, notice: Some(n) }
                    if n.contains("WARNING: trust is off") && n.contains("'o/c:k'")));
            }
        }
        // Unverified + strict + --yes: launch without review or pin, even interactive.
        for interactive in [false, true] {
            let g = gate(Strict, false, interactive, true, "o/c:k", no_review).unwrap();
            assert!(matches!(g, Gate::Launch { pin: false, notice: Some(n) }
                if n.contains("--yes") && n.contains("not pinned")));
        }
        // Unverified + strict + interactive: the review decides; only approval pins.
        let approved = gate(Strict, false, true, false, "o/c:k", || Ok(true)).unwrap();
        assert_eq!(approved, launch(true));
        let rejected = gate(Strict, false, true, false, "o/c:k", || Ok(false)).unwrap();
        assert_eq!(
            rejected,
            Gate::Refuse("Not verified. Refusing to launch.".to_string())
        );
        assert!(gate(Strict, false, true, false, "o/c:k", || anyhow::bail!("tty")).is_err());
        // Unverified + strict + non-interactive: refuse with the ceremony message.
        let g = gate(Strict, false, false, false, "o/c:k", no_review).unwrap();
        assert!(matches!(g, Gate::Refuse(m)
            if m.contains("non-interactive run") && m.contains("nemonik verify o/c:k")));
    }
}
