//! `verify --web`: the kit as a static HTML page, one box per part, with a line diff
//! against the pinned bundle. Headings come from the bundle's structure, never from its
//! text, so kit content can't pass itself off as another part.

use anyhow::Result;
use nemonik_core::mcp::KitBundle;
use nemonik_core::review::{
    display_safe, render_args_text, render_hook_text, render_mcp_text, render_prompt_text,
    render_skill_text,
};
use similar::{ChangeTag, TextDiff};
use std::fmt::Write as _;
use std::path::PathBuf;

const STYLE: &str = "
:root { color-scheme: light dark; }
body { font: 13px ui-monospace, SFMono-Regular, Menlo, monospace; margin: 2rem; }
h2 { font-size: 1rem; margin: 1.5rem 0 0.4rem; }
details, .box { border: 1px solid #8886; border-radius: 6px; margin: 0.5rem 0; }
summary { padding: 0.4rem 0.7rem; background: #8882; cursor: pointer; font-weight: 600; }
pre { margin: 0; padding: 0.5rem 0.7rem; }
pre span { display: block; white-space: pre-wrap; min-height: 1.2em; }
.ins { background: #2ea04340; }
.del { background: #f8514940; }
.tag { font-weight: normal; margin-left: 0.5rem; padding: 0 0.4rem; border-radius: 4px; background: #d2992250; }
";

/// One part of a kit: a heading and its boxes. A box with an empty name is the
/// section's only box and is titled by the section.
struct Section {
    key: &'static str,
    title: String,
    boxes: Vec<(String, String)>,
}

fn sections(bundle: &KitBundle) -> Vec<Section> {
    let mut out = vec![Section {
        key: "prompt",
        title: "Prompt".into(),
        boxes: vec![(String::new(), render_prompt_text(bundle.prompt.as_ref()))],
    }];
    out.push(Section {
        key: "skills",
        title: format!("Skills ({})", bundle.skills.len()),
        boxes: bundle
            .skills
            .iter()
            .map(|s| (display_safe(&s.name), render_skill_text(s)))
            .collect(),
    });
    out.push(Section {
        key: "hooks",
        title: format!("Hooks ({})", bundle.hooks.len()),
        boxes: bundle
            .hooks
            .iter()
            .map(|h| (display_safe(&h.name), render_hook_text(h)))
            .collect(),
    });
    out.push(Section {
        key: "mcps",
        title: format!("MCPs ({}, mode: {:?})", bundle.mcps.len(), bundle.mcp_mode),
        boxes: bundle
            .mcps
            .iter()
            .map(|m| (display_safe(&m.name), render_mcp_text(m)))
            .collect(),
    });
    if !bundle.launch_args.is_empty() {
        out.push(Section {
            key: "launch_args",
            title: "Launch args".into(),
            boxes: vec![(String::new(), render_args_text(&bundle.launch_args))],
        });
    }
    if let Some(wf) = bundle.workflow.as_deref().filter(|s| !s.is_empty()) {
        out.push(Section {
            key: "workflow",
            title: format!("Workflow (inject: {:?})", bundle.workflow_inject),
            boxes: vec![(String::new(), display_safe(wf))],
        });
    }
    out.retain(|s| !s.boxes.is_empty());
    out
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// `None` when there is nothing to compare against or nothing changed.
fn change_tag(previous: Option<&str>, current: &str) -> Option<&'static str> {
    match previous {
        Some(p) if p != current && p.is_empty() => Some("added"),
        Some(p) if p != current && current.is_empty() => Some("removed"),
        Some(p) if p != current => Some("changed"),
        _ => None,
    }
}

fn tag_html(tag: Option<&str>) -> String {
    tag.map(|t| format!("<span class=\"tag\">{}</span>", html_escape(t)))
        .unwrap_or_default()
}

/// The box's lines, marked against `previous` when there is one.
fn render_lines(out: &mut String, previous: Option<&str>, current: &str) {
    out.push_str("<pre>");
    for change in TextDiff::from_lines(previous.unwrap_or(current), current).iter_all_changes() {
        let class = match change.tag() {
            ChangeTag::Delete => "del",
            ChangeTag::Insert => "ins",
            ChangeTag::Equal => "",
        };
        let line = html_escape(change.value().trim_end_matches('\n'));
        let _ = write!(out, "<span class=\"{class}\">{line}</span>");
    }
    out.push_str("</pre>");
}

/// One section: its heading, then one plain box, or one collapsible box per item.
/// `old` is `None` when unpinned, `Some(None)` when the section is new.
fn render_section(
    out: &mut String,
    section: &Section,
    old: Option<Option<&Section>>,
    removed: bool,
) -> bool {
    let single = section.boxes.len() == 1 && section.boxes[0].0.is_empty();
    let prev_text = |name: &str| match old {
        None => None,
        Some(None) => Some(""),
        Some(Some(o)) => Some(
            o.boxes
                .iter()
                .find(|(n, _)| n == name)
                .map_or("", |(_, t)| t.as_str()),
        ),
    };
    let heading_tag = match old {
        _ if removed => Some("removed".to_string()),
        Some(None) => Some("added".to_string()),
        Some(Some(o)) if o.title != section.title => Some(format!("was: {}", o.title)),
        _ if single => change_tag(prev_text(""), &section.boxes[0].1).map(str::to_string),
        _ => None,
    };
    let mut changed = heading_tag.is_some();
    let _ = writeln!(
        out,
        "<h2>{}{}</h2>",
        html_escape(&section.title),
        tag_html(heading_tag.as_deref())
    );
    for (name, text) in &section.boxes {
        let (prev, text) = if removed {
            (Some(text.as_str()), "")
        } else {
            (prev_text(name), text.as_str())
        };
        changed |= prev.is_some_and(|p| p != text);
        if single {
            out.push_str("<div class=\"box\">");
            render_lines(out, prev, text);
            out.push_str("</div>\n");
        } else {
            let _ = write!(
                out,
                "<details open><summary>{}{}</summary>",
                html_escape(name),
                tag_html(change_tag(prev, text))
            );
            render_lines(out, prev, text);
            out.push_str("</details>\n");
        }
    }
    if let Some(Some(o)) = old {
        for (name, text) in o.boxes.iter().filter(|(n, _)| !n.is_empty()) {
            if !section.boxes.iter().any(|(n, _)| n == name) {
                changed = true;
                let _ = write!(
                    out,
                    "<details open><summary>{}{}</summary>",
                    html_escape(name),
                    tag_html(Some("removed"))
                );
                render_lines(out, Some(text), "");
                out.push_str("</details>\n");
            }
        }
    }
    changed
}

fn render_html(
    label: &str,
    previous: Option<&KitBundle>,
    current: &KitBundle,
    note: Option<&str>,
) -> String {
    let now = sections(current);
    let before = previous.map(sections);
    let mut body = String::new();
    let mut any_change = false;

    for section in &now {
        let old = before
            .as_ref()
            .map(|b| b.iter().find(|s| s.key == section.key));
        any_change |= render_section(&mut body, section, old, false);
    }
    for old in before
        .iter()
        .flatten()
        .filter(|o| !now.iter().any(|s| s.key == o.key))
    {
        any_change |= render_section(&mut body, old, None, true);
    }

    let status = match (previous, any_change) {
        (None, _) => "First verification.",
        (Some(_), false) => "No visible changes: a masked MCP value or the kit's id changed.",
        (Some(_), true) => "Changes since last verified.",
    };
    let label = html_escape(label);
    let note = note
        .map(|n| format!("<p>{}</p>\n", html_escape(n)))
        .unwrap_or_default();
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\n<title>{label}</title>\n\
         <style>{STYLE}</style></head><body>\n<h1>{label}</h1>\n<p>{status}</p>\n{note}{body}\
         </body></html>\n"
    )
}

/// Writes the page to `~/.nemonik/last-diff.html` (`0600`) and opens it.
pub fn show(
    label: &str,
    previous: Option<&KitBundle>,
    current: &KitBundle,
    note: Option<&str>,
) -> Result<PathBuf> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let path = nemonik_core::auth::nemonik_dir()?.join("last-diff.html");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    // `mode` applies only on create; an older file keeps its mode otherwise.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(render_html(label, previous, current, note).as_bytes())?;
    nemonik_core::auth::open_browser(&path.to_string_lossy());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nemonik_core::mcp::{McpKind, McpSpec, PromptSpec, SkillSpec};

    fn bundle(prompt: &str, skills: &[(&str, &str)]) -> KitBundle {
        KitBundle {
            id: None,
            prompt: Some(PromptSpec {
                content: prompt.into(),
            }),
            skills: skills
                .iter()
                .map(|(n, b)| SkillSpec {
                    name: n.to_string(),
                    description: "d".into(),
                    body: b.to_string(),
                })
                .collect(),
            mcps: vec![],
            mcp_mode: Default::default(),
            hooks: vec![],
            launch_args: vec![],
            workflow: None,
            workflow_inject: Default::default(),
        }
    }

    #[test]
    fn kit_text_cannot_pose_as_a_heading_or_markup() {
        let html = render_html(
            "k",
            None,
            &bundle("## Skills\n<script>x</script>\n", &[]),
            None,
        );
        assert!(!html.contains("<script>"), "{html}");
        assert_eq!(html.matches("<h2>").count(), 1, "only Prompt: {html}");
        assert!(html.contains("<span class=\"\">## Skills</span>"), "{html}");
    }

    #[test]
    fn every_part_gets_its_own_box_with_secrets_masked() {
        let mut b = bundle("p", &[("gh", "body")]);
        b.mcps = vec![McpSpec {
            name: "svc".into(),
            kind: McpKind::Stdio,
            command: Some("sh".into()),
            args: vec![],
            url: None,
            headers: Default::default(),
            env: [("TOKEN".to_string(), "s3cret".to_string())].into(),
        }];
        b.launch_args = vec!["--model".into(), "x".into()];
        b.workflow = Some("first do this".into());
        let html = render_html("k", None, &b, None);
        for part in [
            "<h2>Prompt</h2>",
            "<h2>Skills (1)</h2>",
            "<summary>gh</summary>",
            "<h2>MCPs (1, mode: Extend)</h2>",
            "<h2>Launch args</h2>",
            "<h2>Workflow (inject: Both)</h2>",
        ] {
            assert!(html.contains(part), "missing {part}: {html}");
        }
        assert!(html.contains("TOKEN") && !html.contains("s3cret"), "{html}");
        assert!(html.contains("First verification."), "{html}");
    }

    #[test]
    fn a_pin_with_no_visible_change_points_at_masked_values_or_the_id() {
        let b = bundle("p", &[("gh", "body")]);
        let html = render_html("k", Some(&b), &b, None);
        assert!(
            html.contains("No visible changes: a masked MCP value or the kit's id changed."),
            "{html}"
        );
    }

    #[test]
    fn a_diff_marks_changed_added_and_removed_boxes() {
        let mut before = bundle("p", &[("gh", "old"), ("gone", "bye")]);
        before.workflow = Some("wf".into());
        let mut after = bundle("p", &[("gh", "new"), ("added", "hi")]);
        after.workflow = Some("wf".into());
        after.workflow_inject = nemonik_core::mcp::WorkflowInject::User;
        let html = render_html("k", Some(&before), &after, Some("id changed"));
        assert!(html.contains("Changes since last verified."), "{html}");
        for part in [
            "<summary>gh<span class=\"tag\">changed</span>",
            "<summary>added<span class=\"tag\">added</span>",
            "<summary>gone<span class=\"tag\">removed</span>",
            "<span class=\"del\">old</span>",
            "<span class=\"ins\">new</span>",
            "<h2>Workflow (inject: User)<span class=\"tag\">was: Workflow (inject: Both)</span></h2>",
            "<h2>Prompt</h2>",
            "<p>id changed</p>",
        ] {
            assert!(html.contains(part), "missing {part}: {html}");
        }
    }
}
