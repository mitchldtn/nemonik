//! Reads a local kit's parts (prompt, skills, MCPs, hooks) from disk.

use crate::mcp::{HookSpec, McpSpec, PromptSpec, SkillSpec};
use anyhow::{Context, Result};
use std::path::Path;

pub fn read_prompt(path: &Path) -> Result<PromptSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read local prompt file {}", path.display()))?;
    Ok(PromptSpec { content })
}

pub fn read_workflow(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("failed to read local workflow file {}", path.display()))
}

/// Reads a skills directory: each subdirectory with a `SKILL.md` becomes a skill
/// named after the subdirectory.
pub fn read_skills_dir(dir: &Path) -> Result<Vec<SkillSpec>> {
    let mut skills = Vec::new();
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read local skills directory {}", dir.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read an entry in {}", dir.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.is_file() {
            continue;
        }
        let body = std::fs::read_to_string(&skill_md)
            .with_context(|| format!("failed to read {}", skill_md.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let description = frontmatter_field(&body, "description").unwrap_or_default();
        skills.push(SkillSpec {
            name,
            description,
            body,
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

pub fn read_mcps_file(path: &Path) -> Result<Vec<McpSpec>> {
    read_json_array(path, "MCP")
}

pub fn read_hooks_file(path: &Path) -> Result<Vec<HookSpec>> {
    read_json_array(path, "hook")
}

fn read_json_array<T: serde::de::DeserializeOwned>(path: &Path, what: &str) -> Result<Vec<T>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read local {what} file {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("{} must be a JSON array of {what} specs", path.display()))
}

fn frontmatter_field(body: &str, key: &str) -> Option<String> {
    let rest = body.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    for line in rest[..end].lines() {
        if let Some(after_key) = line.trim().strip_prefix(key)
            && let Some(value) = after_key.trim_start().strip_prefix(':')
        {
            return Some(
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim()
                    .to_string(),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir {
        _dir: tempfile::TempDir,
        root: PathBuf,
    }
    impl TempDir {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            Self {
                root: dir.path().to_path_buf(),
                _dir: dir,
            }
        }
        fn write(&self, rel: &str, contents: &str) -> PathBuf {
            let p = self.root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, contents).unwrap();
            p
        }
    }

    #[test]
    fn read_skills_dir_reads_each_subdir_with_a_skill_md_sorted_by_name() {
        let t = TempDir::new();
        t.write(
            "skills/gh/SKILL.md",
            "---\nname: gh\ndescription: Use the GitHub CLI\n---\n\n# gh\nbody\n",
        );
        t.write(
            "skills/aws/SKILL.md",
            "---\ndescription: 'AWS helper'\n---\n# aws\n",
        );
        t.write("skills/not-a-skill/README.md", "no SKILL.md here");
        let skills = read_skills_dir(&t.root.join("skills")).unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "aws");
        assert_eq!(skills[0].description, "AWS helper");
        assert_eq!(skills[1].name, "gh");
        assert_eq!(skills[1].description, "Use the GitHub CLI");
        assert!(skills[1].body.contains("# gh"));
        assert!(skills[1].body.starts_with("---"));
    }

    #[test]
    fn read_skills_dir_defaults_description_to_empty_when_no_frontmatter() {
        let t = TempDir::new();
        t.write(
            "skills/plain/SKILL.md",
            "# plain\njust a body, no frontmatter\n",
        );
        let skills = read_skills_dir(&t.root.join("skills")).unwrap();
        assert_eq!(skills[0].name, "plain");
        assert_eq!(skills[0].description, "");
    }

    #[test]
    fn read_json_array_errors_with_a_helpful_message_on_malformed_content() {
        let t = TempDir::new();
        let p = t.write("mcps.json", "{ not an array }");
        let err = read_mcps_file(&p).unwrap_err().to_string();
        assert!(err.contains("must be a JSON array"), "got: {err}");
    }
}
