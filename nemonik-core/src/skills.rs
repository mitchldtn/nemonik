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

/// The Codex home a session started from a shell with `CODEX_HOME=home` should use.
/// A nemonik session home is removed when its session ends, so it passes on the home it
/// was seeded from, where its `auth.json` link points; `None` (Codex's default) without
/// that link.
fn codex_home_to_pass_on(home: &Path, sessions: &Path) -> Option<PathBuf> {
    let resolved = |p: &Path| std::fs::canonicalize(p).unwrap_or(p.to_path_buf());
    if !resolved(home).starts_with(resolved(sessions)) {
        return Some(home.to_path_buf());
    }
    let auth = std::fs::read_link(home.join("auth.json")).ok()?;
    auth.parent().map(Path::to_path_buf)
}

/// The launching shell's `CODEX_HOME` to put in a session's spec, when it has one.
pub fn launching_codex_home() -> Result<Option<PathBuf>> {
    let Some(home) = std::env::var_os("CODEX_HOME") else {
        return Ok(None);
    };
    Ok(codex_home_to_pass_on(Path::new(&home), &sessions_dir()?))
}

fn real_codex_home() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(dir));
    }
    Ok(crate::auth::home_dir()?.join(".codex"))
}

/// `config` with every hook trusted under `<from>/hooks.json` also trusted under
/// `<to>/hooks.json`. Codex keys a hook's trust by its file's path and the hook's hash,
/// so a copied `hooks.json` is trusted only under its new path; Codex still checks each
/// hash.
fn with_hook_trust_moved(config: &str, from: &Path, to: &Path) -> Result<String> {
    let mut doc: toml_edit::DocumentMut = config.parse()?;
    let Some(state) = doc
        .get_mut("hooks")
        .and_then(|hooks| hooks.get_mut("state"))
        .and_then(|state| state.as_table_like_mut())
    else {
        return Ok(config.to_string());
    };
    let from = format!("{}/hooks.json:", from.display());
    let to = format!("{}/hooks.json:", to.display());
    let moved: Vec<_> = state
        .iter()
        .filter_map(|(key, entry)| {
            let hook = key.strip_prefix(&from)?;
            Some((format!("{to}{hook}"), entry.clone()))
        })
        .collect();
    for (key, entry) in moved {
        state.insert(&key, entry);
    }
    Ok(doc.to_string())
}

// Seeds a per-session Codex home: `auth.json` and `skills/.system` are symlinked
// (token refreshes write through), `config.toml` and `hooks.json` are copied so session
// edits never reach the real ones. Links point at the fully resolved file: a launch from
// inside a Codex session inherits that session's `CODEX_HOME`, which is removed when it
// ends.
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
    let copy_in = |rel: &str| -> Result<()> {
        let src = real.join(rel);
        if !src.exists() {
            return Ok(());
        }
        let target = dest.join(rel);
        let _ = std::fs::remove_file(&target);
        std::fs::copy(&src, &target)
            .map(|_| ())
            .with_context(|| format!("failed to copy codex {rel} into '{}'", target.display()))
    };
    copy_in("config.toml")?;
    copy_in("hooks.json")?;
    let config_dest = dest.join("config.toml");
    if dest.join("hooks.json").exists() && config_dest.exists() {
        // Codex names a home by its resolved path.
        let resolved = |home: &Path| std::fs::canonicalize(home).unwrap_or(home.to_path_buf());
        let config = std::fs::read_to_string(&config_dest)
            .with_context(|| format!("failed to read '{}'", config_dest.display()))?;
        let config = with_hook_trust_moved(&config, &resolved(real), &resolved(dest))
            .with_context(|| format!("failed to parse '{}'", real.join("config.toml").display()))?;
        std::fs::write(&config_dest, config)
            .with_context(|| format!("failed to write '{}'", config_dest.display()))?;
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
        assert!(!dest.path().join("hooks.json").exists());
        assert!(!dest.path().join("skills/.system").exists());
        assert!(dest.path().join("skills").is_dir());
    }

    #[test]
    fn codex_home_to_pass_on_is_the_origin_of_a_session_home() {
        let real = TempDir::new().unwrap();
        let sessions = TempDir::new().unwrap();
        std::fs::write(real.path().join("auth.json"), "tok").unwrap();
        let parent = sessions.path().join("parent/codex-home");
        seed_codex_home(real.path(), &parent).unwrap();
        let child = sessions.path().join("child/codex-home");
        seed_codex_home(&parent, &child).unwrap();
        let origin = std::fs::canonicalize(real.path()).unwrap();

        for home in [&parent, &child] {
            assert_eq!(
                codex_home_to_pass_on(home, sessions.path()),
                Some(origin.clone())
            );
        }
        assert_eq!(
            codex_home_to_pass_on(real.path(), sessions.path()),
            Some(real.path().to_path_buf())
        );
        let keyring_login = sessions.path().join("k/codex-home");
        std::fs::create_dir_all(&keyring_login).unwrap();
        assert_eq!(codex_home_to_pass_on(&keyring_login, sessions.path()), None);
    }

    #[test]
    fn seed_codex_home_copies_hooks_and_trusts_them_in_the_new_home_too() {
        let real = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        let real_dir = std::fs::canonicalize(real.path()).unwrap();
        let dest_dir = std::fs::canonicalize(dest.path()).unwrap();
        let key = |home: &Path| format!("{}/hooks.json:stop:0:0", home.display());
        std::fs::write(real.path().join("hooks.json"), "{\"hooks\":{}}").unwrap();
        std::fs::write(
            real.path().join("config.toml"),
            format!(
                "model = \"m\"\n\n[hooks.state.\"{}\"]\nenabled = true\ntrusted_hash = \"sha256:aa\"\n\n[hooks.state.\"/other/hooks.json:stop:0:0\"]\ntrusted_hash = \"sha256:bb\"\n",
                key(&real_dir)
            ),
        )
        .unwrap();

        seed_codex_home(real.path(), dest.path()).unwrap();
        seed_codex_home(real.path(), dest.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(dest.path().join("hooks.json")).unwrap(),
            "{\"hooks\":{}}"
        );
        assert!(
            !std::fs::symlink_metadata(dest.path().join("hooks.json"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let seeded: toml_edit::DocumentMut =
            std::fs::read_to_string(dest.path().join("config.toml"))
                .unwrap()
                .parse()
                .unwrap();
        let state = seeded["hooks"]["state"].as_table_like().unwrap();
        assert_eq!(state.len(), 3, "the copy is added once, the rest kept");
        for home in [&real_dir, &dest_dir] {
            let entry = state.get(&key(home)).unwrap();
            assert_eq!(entry["trusted_hash"].as_str(), Some("sha256:aa"));
            assert_eq!(entry["enabled"].as_bool(), Some(true));
        }
        assert_eq!(seeded["model"].as_str(), Some("m"));
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
