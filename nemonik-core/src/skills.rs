//! Writes a kit's skills under a per-session `~/.nemonik/sessions/<session>/` base
//! (outside any repo); cleanup removes the whole base.

use crate::mcp::SkillSpec;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn sessions_dir() -> Result<PathBuf> {
    Ok(crate::auth::nemonik_dir()?.join("sessions"))
}

/// A launch's per-session base, `~/.nemonik/sessions/<session>`, passed to the agent
/// with `--add-dir`.
pub fn session_base(session: &str) -> Result<PathBuf> {
    Ok(sessions_dir()?.join(session))
}

/// Which agent's skill layout to write: Claude's `.claude/skills/` or Codex's
/// `CODEX_HOME/skills/`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillLayout {
    #[default]
    Claude,
    Codex,
}

fn skills_root_in(base: &Path, layout: SkillLayout) -> PathBuf {
    match layout {
        SkillLayout::Claude => base.join(".claude").join("skills"),
        SkillLayout::Codex => base.join("codex-home").join("skills"),
    }
}

/// A Codex launch's isolated per-session `CODEX_HOME`.
pub fn codex_home_dir(session: &str) -> Result<PathBuf> {
    Ok(session_base(session)?.join("codex-home"))
}

fn real_codex_home() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(dir));
    }
    Ok(crate::auth::home_dir()?.join(".codex"))
}

// Seeds a per-session Codex home: `auth.json` and `skills/.system` are symlinked
// (token refreshes write through), `config.toml` is copied so session edits never
// reach the real config. Links point at the fully resolved file: a launch from inside
// a Codex session inherits that session's `CODEX_HOME`, which is removed when it ends.
fn seed_codex_home(real: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest.join("skills"))
        .with_context(|| format!("failed to create codex home '{}'", dest.display()))?;
    let symlink_in = |rel: &str, target: PathBuf| -> Result<()> {
        let Ok(src) = std::fs::canonicalize(real.join(rel)) else {
            return Ok(());
        };
        let _ = std::fs::remove_file(&target);
        std::os::unix::fs::symlink(&src, &target).with_context(|| {
            format!(
                "failed to symlink codex '{rel}' into '{}'",
                target.display()
            )
        })
    };
    symlink_in("auth.json", dest.join("auth.json"))?;
    symlink_in("skills/.system", dest.join("skills").join(".system"))?;
    let config_src = real.join("config.toml");
    if config_src.exists() {
        let config_dest = dest.join("config.toml");
        let _ = std::fs::remove_file(&config_dest);
        std::fs::copy(&config_src, &config_dest).with_context(|| {
            format!(
                "failed to copy codex config.toml into '{}'",
                config_dest.display()
            )
        })?;
    }
    Ok(())
}

// A `.system` skill would write through the seeded symlink into the real ~/.codex.
fn codex_reserved_skill_conflict(skills: &[SkillSpec]) -> bool {
    skills.iter().any(|s| s.name == ".system")
}

// Synthesizes `name`/`description` frontmatter unless the body has its own: Codex
// won't register a SKILL.md without `name:` or with an empty `description:`.
fn skill_md_content(skill: &SkillSpec) -> String {
    if body_has_frontmatter(&skill.body) {
        return skill.body.clone();
    }
    let description = if skill.description.trim().is_empty() {
        &skill.name
    } else {
        &skill.description
    };
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}",
        yaml_double_quote(&skill.name),
        yaml_double_quote(description),
        skill.body
    )
}

// Only a closed block with a `name:` key counts; anything else gets synthesized frontmatter.
fn body_has_frontmatter(body: &str) -> bool {
    let b = body.strip_prefix('\u{feff}').unwrap_or(body);
    let Some(rest) = b
        .strip_prefix("---\n")
        .or_else(|| b.strip_prefix("---\r\n"))
    else {
        return false;
    };
    let mut has_name = false;
    for line in rest.lines() {
        let line = line.trim_end_matches('\r');
        if line == "---" || line == "..." {
            return has_name;
        }
        if line.trim_start().starts_with("name:") {
            has_name = true;
        }
    }
    false
}

fn yaml_double_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // C0 + C1 control chars (incl. U+0085 NEL) → `\xNN` (both ≤ 0xFF).
            c if (c as u32) < 0x20 || (0x7f..=0x9f).contains(&(c as u32)) => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            // Strict YAML disallows raw U+2028/U+2029 in a quoted scalar.
            '\u{2028}' | '\u{2029}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn remove_session_base(base: &Path) -> Result<()> {
    match std::fs::remove_dir_all(base) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            Err(e).with_context(|| format!("failed to remove session base '{}'", base.display()))
        }
    }
}

// A single normal path component. `skill.name` comes from the kit, and `join` with an
// absolute or `..` value would escape the skills root.
fn is_safe_path_component(component: &str) -> bool {
    if component.is_empty() {
        return false;
    }
    let mut components = Path::new(component).components();
    matches!(components.next(), Some(std::path::Component::Normal(c)) if c == std::ffi::OsStr::new(component))
        && components.next().is_none()
}

fn check_names(session: &str, skills: &[SkillSpec]) -> Result<()> {
    if !is_safe_path_component(session) {
        anyhow::bail!("refusing to materialize skills: '{session}' is not a safe session name");
    }
    for skill in skills {
        if !is_safe_path_component(&skill.name) {
            anyhow::bail!(
                "refusing to materialize skill '{}': not a safe directory name (no '/', \
                 no '..', not absolute, not empty)",
                skill.name
            );
        }
    }
    Ok(())
}

fn materialize_into(root: &Path, skills: &[SkillSpec]) -> Result<()> {
    for skill in skills {
        let dir = root.join(&skill.name);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create skill directory '{}'", dir.display()))?;
        std::fs::write(dir.join("SKILL.md"), skill_md_content(skill))
            .with_context(|| format!("failed to write SKILL.md into '{}'", dir.display()))?;
    }
    Ok(())
}

pub fn materialize(skills: &[SkillSpec], session: &str, layout: SkillLayout) -> Result<()> {
    // Every name is checked before anything is written, so a bad name leaves nothing behind.
    check_names(session, skills)?;
    if layout == SkillLayout::Codex {
        if codex_reserved_skill_conflict(skills) {
            anyhow::bail!(
                "a kit skill named '.system' can't be materialized for --agent codex: '.system' is \
                 Codex's reserved built-in-skills directory (seeded as a symlink to your real \
                 ~/.codex), and writing to it would reach the real home. Rename the skill, or use \
                 --agent claude."
            );
        }
        seed_codex_home(&real_codex_home()?, &codex_home_dir(session)?)?;
    }
    materialize_into(&skills_root_in(&session_base(session)?, layout), skills)
}

/// Removes a session's materialized skills: its whole session base.
pub fn cleanup(session: &str) -> Result<()> {
    if !is_safe_path_component(session) {
        anyhow::bail!("refusing to clean up skills: '{session}' is not a safe session name");
    }
    remove_session_base(&session_base(session)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn skill(name: &str, body: &str) -> SkillSpec {
        SkillSpec {
            name: name.to_string(),
            description: "d".to_string(),
            body: body.to_string(),
        }
    }

    #[test]
    fn seed_codex_home_symlinks_auth_and_system_but_copies_config() {
        let real = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        std::fs::write(real.path().join("auth.json"), "tok").unwrap();
        std::fs::write(real.path().join("config.toml"), "cfg").unwrap();
        std::fs::create_dir_all(real.path().join("skills/.system/imagegen")).unwrap();

        seed_codex_home(real.path(), dest.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(dest.path().join("auth.json")).unwrap(),
            "tok"
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("config.toml")).unwrap(),
            "cfg"
        );
        assert!(dest.path().join("skills/.system/imagegen").exists());
        assert!(
            std::fs::symlink_metadata(dest.path().join("auth.json"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            std::fs::symlink_metadata(dest.path().join("skills/.system"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            !std::fs::symlink_metadata(dest.path().join("config.toml"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn seed_codex_home_from_a_session_home_outlives_that_session() {
        let real = TempDir::new().unwrap();
        let parent = TempDir::new().unwrap();
        let child = TempDir::new().unwrap();
        std::fs::write(real.path().join("auth.json"), "tok").unwrap();
        std::fs::create_dir_all(real.path().join("skills/.system/imagegen")).unwrap();
        seed_codex_home(real.path(), parent.path()).unwrap();
        std::fs::write(parent.path().join("config.toml"), "parent-cfg").unwrap();

        seed_codex_home(parent.path(), child.path()).unwrap();
        for rel in ["auth.json", "skills/.system"] {
            assert_eq!(
                std::fs::read_link(child.path().join(rel)).unwrap(),
                std::fs::canonicalize(real.path().join(rel)).unwrap()
            );
        }
        drop(parent);

        assert_eq!(
            std::fs::read_to_string(child.path().join("auth.json")).unwrap(),
            "tok"
        );
        assert!(child.path().join("skills/.system/imagegen").exists());
        assert_eq!(
            std::fs::read_to_string(child.path().join("config.toml")).unwrap(),
            "parent-cfg"
        );
    }

    #[test]
    fn seed_codex_home_is_re_seedable() {
        let real = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        std::fs::write(real.path().join("auth.json"), "tok").unwrap();
        std::fs::write(real.path().join("config.toml"), "cfg").unwrap();
        seed_codex_home(real.path(), dest.path()).unwrap();
        seed_codex_home(real.path(), dest.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.path().join("auth.json")).unwrap(),
            "tok"
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("config.toml")).unwrap(),
            "cfg"
        );
    }

    #[test]
    fn seed_codex_home_skips_absent_sources_but_still_makes_the_skills_dir() {
        let real = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        std::fs::write(real.path().join("auth.json"), "tok").unwrap();

        seed_codex_home(real.path(), dest.path()).unwrap();

        assert!(dest.path().join("auth.json").exists());
        assert!(!dest.path().join("config.toml").exists());
        assert!(!dest.path().join("skills/.system").exists());
        assert!(dest.path().join("skills").is_dir());
    }

    #[test]
    fn skill_md_content_synthesizes_frontmatter_before_the_body() {
        let s = SkillSpec {
            name: "trading-rules".to_string(),
            description: "Pre-trade checklist.".to_string(),
            body: "# Rules\n\nDo the thing.\n".to_string(),
        };
        assert_eq!(
            skill_md_content(&s),
            "---\nname: \"trading-rules\"\ndescription: \"Pre-trade checklist.\"\n---\n\n# Rules\n\nDo the thing.\n"
        );
    }

    #[test]
    fn yaml_double_quote_escapes_quotes_backslashes_newlines_and_controls() {
        assert_eq!(yaml_double_quote("plain"), "\"plain\"");
        assert_eq!(yaml_double_quote("a: b # c"), "\"a: b # c\""); // colon/hash safe inside quotes
        assert_eq!(
            yaml_double_quote("say \"hi\"\\n"),
            "\"say \\\"hi\\\"\\\\n\""
        );
        assert_eq!(yaml_double_quote("line1\nline2"), "\"line1\\nline2\"");
        assert_eq!(yaml_double_quote("\u{7f}\u{85}"), "\"\\x7f\\x85\""); // DEL + C1 NEL
        assert_eq!(yaml_double_quote("a\u{2028}b"), "\"a\\u2028b\""); // line separator
    }

    #[test]
    fn skill_md_content_uses_the_name_for_an_empty_description() {
        let content = skill_md_content(&SkillSpec {
            name: "plain-notes".to_string(),
            description: " ".to_string(),
            body: "# Notes\n".to_string(),
        });
        assert!(
            content.contains("\ndescription: \"plain-notes\"\n"),
            "{content}"
        );
    }

    #[test]
    fn skill_md_content_passes_a_body_that_already_has_frontmatter_through_unchanged() {
        let body = "---\nname: custom\ndescription: already here\n---\n\n# Body\n";
        let s = SkillSpec {
            name: "x".to_string(),
            description: "d".to_string(),
            body: body.to_string(),
        };
        assert_eq!(skill_md_content(&s), body);
    }

    #[test]
    fn body_has_frontmatter_requires_a_closed_block_with_a_name_key() {
        assert!(body_has_frontmatter(
            "---\nname: x\ndescription: y\n---\n\nbody"
        ));
        assert!(body_has_frontmatter("---\r\nname: x\r\n---\r\nbody")); // CRLF
        assert!(body_has_frontmatter("\u{feff}---\nname: x\n---\n")); // tolerates a BOM
        assert!(!body_has_frontmatter("---\nname: x\n")); // unterminated
        assert!(!body_has_frontmatter("---\nfoo: bar\n---\n")); // no name key
        assert!(!body_has_frontmatter("---\nsome prose\n---\n")); // thematic-break-ish, no name
        assert!(!body_has_frontmatter("# Heading\n---\n")); // a `---` further down is NOT frontmatter
        assert!(!body_has_frontmatter("no marker"));
        assert!(!body_has_frontmatter(""));
    }

    #[test]
    fn remove_session_base_removes_the_base_and_is_idempotent() {
        let base = TempDir::new().unwrap();
        let path = base.path().to_path_buf();
        std::fs::create_dir_all(path.join(".claude/skills/gh")).unwrap();
        std::fs::write(path.join(".claude/skills/gh/SKILL.md"), "body").unwrap();
        remove_session_base(&path).unwrap();
        assert!(!path.exists());
        remove_session_base(&path).unwrap();
    }

    #[test]
    fn materialize_into_writes_each_skill_md() {
        let root = TempDir::new().unwrap();
        let skills = vec![skill("gh", "gh body"), skill("home-k8s", "k8s body")];
        materialize_into(root.path(), &skills).unwrap();

        for s in &skills {
            assert_eq!(
                std::fs::read_to_string(root.path().join(&s.name).join("SKILL.md")).unwrap(),
                skill_md_content(s)
            );
        }
    }

    #[test]
    fn is_safe_path_component_accepts_plain_names_and_rejects_escapes() {
        assert!(is_safe_path_component("gh"));
        assert!(is_safe_path_component("home-k8s"));
        for bad in [
            "",
            "..",
            "../../../.ssh",
            "safe/../../../etc",
            "/etc/cron.d/evil",
            "a/b",
        ] {
            assert!(!is_safe_path_component(bad), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn check_names_rejects_an_unsafe_session_or_skill_name() {
        let ok = [skill("gh", "b"), skill("home-k8s", "b")];
        assert!(check_names("sess", &ok).is_ok());

        let err = check_names("../escape", &ok).unwrap_err().to_string();
        assert!(err.contains("not a safe session name"), "{err}");
        for bad in ["../../canary", "/tmp/canary"] {
            let skills = [skill("gh", "b"), skill(bad, "evil")];
            let err = check_names("sess", &skills).unwrap_err().to_string();
            assert!(err.contains("safe directory name"), "{bad}: {err}");
        }
    }

    #[test]
    fn cleanup_rejects_an_unsafe_session_name() {
        assert!(cleanup("../escape").is_err());
    }
}
