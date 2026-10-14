//! Kit review text: escaped for display, with MCP env and header values masked.

use crate::mcp::{HookSpec, McpSpec, PromptSpec, SkillSpec};
use std::fmt::Write as _;

/// Escapes control and bidi-override characters for display. Display only:
/// hashing and pinning use the raw bytes.
pub fn display_safe(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for c in content.chars() {
        let code = c as u32;
        match c {
            '\n' | '\t' => out.push(c),
            _ if code < 0x20 || code == 0x7F => out.push_str(&format!("\\x{code:02x}")),
            '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' => {
                out.push_str(&format!("\\u{{{code:04x}}}"))
            }
            _ => out.push(c),
        }
    }
    out
}

pub fn render_prompt_text(prompt: Option<&PromptSpec>) -> String {
    match prompt {
        Some(prompt) => display_safe(&prompt.content),
        None => "(none: this kit has no linked prompt)".to_string(),
    }
}

pub fn render_skill_text(skill: &SkillSpec) -> String {
    format!(
        "{}\n{}\n{}",
        display_safe(&skill.name),
        display_safe(&skill.description),
        display_safe(&skill.body)
    )
}

/// One MCP's review text. `env` and `headers` values are always masked to key names.
pub fn render_mcp_text(mcp: &McpSpec) -> String {
    let mut out = format!("{} ({:?})", display_safe(&mcp.name), mcp.kind);
    if let Some(command) = &mcp.command {
        let _ = write!(out, "\ncommand: {}", display_safe(command));
    }
    if !mcp.args.is_empty() {
        let _ = write!(out, "\nargs:\n{}", render_args_text(&mcp.args));
    }
    if let Some(url) = &mcp.url {
        let _ = write!(out, "\nurl: {}", display_safe(url));
    }
    if !mcp.headers.is_empty() {
        let keys = mcp
            .headers
            .keys()
            .map(|k| display_safe(k))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, "\nheaders (values masked): {keys}");
    }
    if !mcp.env.is_empty() {
        let keys = mcp
            .env
            .keys()
            .map(|k| display_safe(k))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, "\nenv (values masked): {keys}");
    }
    out
}

/// One numbered, quoted token per line: order and boundaries are what the hash pins.
pub fn render_args_text(args: &[String]) -> String {
    args.iter()
        .enumerate()
        .map(|(i, arg)| format!("  {}: '{}'", i + 1, display_safe(arg)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One hook's review text. `config` is shown in full, unmasked: it is shell a
/// reviewer must be able to read before pinning.
pub fn render_hook_text(hook: &HookSpec) -> String {
    let mut out = display_safe(&hook.name);
    if let Some(desc) = &hook.description {
        let _ = write!(out, "\n{}", display_safe(desc));
    }
    let config =
        serde_json::to_string_pretty(&hook.config).unwrap_or_else(|_| hook.config.to_string());
    let _ = write!(out, "\n{}", display_safe(&config));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{McpKind, McpSpec};

    #[test]
    fn display_safe_escapes_ansi_escape_byte_instead_of_letting_it_render() {
        assert_eq!(display_safe("a\x1bb"), "a\\x1bb");
    }

    #[test]
    fn display_safe_escapes_bidi_override_characters() {
        // U+202E is RIGHT-TO-LEFT OVERRIDE.
        assert_eq!(display_safe("a\u{202E}b"), "a\\u{202e}b");
    }

    fn mcp_with_env(env: &[(&str, &str)], headers: &[(&str, &str)]) -> McpSpec {
        McpSpec {
            name: "svc".to_string(),
            kind: McpKind::Stdio,
            command: None,
            args: vec![],
            url: None,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn render_mcp_text_shows_where_each_arg_starts_and_ends() {
        let with_args = |args: &[&str]| McpSpec {
            command: Some("sh".to_string()),
            args: args.iter().map(|a| a.to_string()).collect(),
            ..mcp_with_env(&[], &[])
        };
        let two = render_mcp_text(&with_args(&["-c", "echo hello"]));
        let three = render_mcp_text(&with_args(&["-c", "echo", "hello"]));
        assert_ne!(two, three);
        assert!(two.contains("  2: 'echo hello'"), "{two}");
    }

    #[test]
    fn render_mcp_text_masks_env_and_header_values_to_key_names_only() {
        let text = render_mcp_text(&mcp_with_env(
            &[("TOKEN", "s3cret-value")],
            &[("Authorization", "Bearer xyz")],
        ));
        assert!(text.contains("TOKEN"), "key shown: {text}");
        assert!(text.contains("Authorization"), "header key shown: {text}");
        assert!(
            !text.contains("s3cret-value"),
            "env value must be masked: {text}"
        );
        assert!(
            !text.contains("Bearer xyz"),
            "header value must be masked: {text}"
        );
    }
}
