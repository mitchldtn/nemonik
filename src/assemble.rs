//! Assembles a `nemonik.json` local kit into the same `KitBundle` a server fetch
//! returns. A local kit is local only: every part is a file, so it needs no login.

use anyhow::Result;
use nemonik_core::local_kit;
use nemonik_core::mcp::{self, KitBundle};
use nemonik_core::project_config::{LocalKitDef, is_local_path, resolve_local_path};
use std::path::{Path, PathBuf};

/// The kit bundle for `verify`/`run`: a `kits` entry assembled from disk, else a server
/// fetch.
pub async fn resolve_bundle(name: &str) -> Result<KitBundle> {
    let (config, base) = nemonik_core::project_config::discover()?;
    if let Some(def) = config.kits.get(name) {
        let base = base.ok_or_else(|| {
            anyhow::anyhow!("internal: local kit '{name}' resolved with no config directory")
        })?;
        eprintln!("nemonik: assembling local kit '{name}' from nemonik.json");
        return assemble_local_kit(name, def, &base);
    }
    mcp::fetch_kit_bundle(name).await
}

pub fn assemble_local_kit(name: &str, def: &LocalKitDef, base: &Path) -> Result<KitBundle> {
    let prompt = match &def.prompt {
        None => None,
        Some(v) => Some(local_kit::read_prompt(&local_part_path(
            name, "prompt", base, v,
        )?)?),
    };

    let mut skills = Vec::new();
    for entry in &def.skills {
        skills.extend(local_kit::read_skills_dir(&local_part_path(
            name, "skill", base, entry,
        )?)?);
    }
    warn_duplicate_names(name, "skill", skills.iter().map(|s| s.name.as_str()));

    let mut mcps = Vec::new();
    for entry in &def.mcps {
        mcps.extend(local_kit::read_mcps_file(&local_part_path(
            name, "mcp", base, entry,
        )?)?);
    }
    warn_duplicate_names(name, "mcp", mcps.iter().map(|m| m.name.as_str()));
    if mcps
        .iter()
        .any(|m| !m.env.is_empty() || !m.headers.is_empty())
    {
        eprintln!(
            "nemonik: warning: local kit '{name}': MCP env and header values are masked in \
             the review; read them in the kit's MCP files before approving."
        );
    }

    let mut hooks = Vec::new();
    for entry in &def.hooks {
        hooks.extend(local_kit::read_hooks_file(&local_part_path(
            name, "hook", base, entry,
        )?)?);
    }
    warn_duplicate_names(name, "hook", hooks.iter().map(|h| h.name.as_str()));

    let workflow = match &def.workflow {
        None => None,
        Some(v) => Some(local_kit::read_workflow(&local_part_path(
            name, "workflow", base, v,
        )?)?),
    };
    Ok(KitBundle {
        id: None,
        prompt,
        skills,
        mcps,
        mcp_mode: def.mcp_mode.unwrap_or_default(),
        hooks,
        launch_args: def.launch_args.clone(),
        workflow,
        workflow_inject: def.workflow_inject.unwrap_or_default(),
    })
}

fn local_part_path(kit: &str, kind: &str, base: &Path, v: &str) -> Result<PathBuf> {
    if !is_local_path(v) {
        anyhow::bail!(
            "local kit '{kit}': {kind} '{v}' must be a local path starting with ./, ../, / or ~"
        );
    }
    Ok(resolve_local_path(base, v))
}

fn warn_duplicate_names<'a>(kit: &str, kind: &str, names: impl Iterator<Item = &'a str>) {
    let mut seen = std::collections::HashSet::new();
    let dups: std::collections::BTreeSet<&str> = names.filter(|n| !seen.insert(*n)).collect();
    if !dups.is_empty() {
        let list = dups.into_iter().collect::<Vec<_>>().join(", ");
        eprintln!("nemonik: warning: local kit '{kit}' has duplicate {kind} names: {list}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nemonik_core::mcp::{McpMode, WorkflowInject};
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
        fn write(&self, rel: &str, contents: &str) {
            let p = self.root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, contents).unwrap();
        }
    }

    fn def() -> LocalKitDef {
        LocalKitDef::default()
    }

    #[test]
    fn assembles_a_local_kit_from_files() {
        let t = TempDir::new();
        t.write("prompt.md", "## Do the thing\n");
        t.write(
            "skills/gh/SKILL.md",
            "---\ndescription: gh helper\n---\n# gh\n",
        );
        t.write(
            "mcps.json",
            r#"[{"name":"fmt","type":"stdio","command":"sh","args":[],"env":{}}]"#,
        );

        let d = LocalKitDef {
            prompt: Some("./prompt.md".into()),
            skills: vec!["./skills".into()],
            mcps: vec!["./mcps.json".into()],
            launch_args: vec!["--model".into(), "fable".into()],
            mcp_mode: Some(McpMode::Only),
            ..def()
        };
        let bundle = assemble_local_kit("mine", &d, &t.root).unwrap();
        assert_eq!(bundle.prompt.unwrap().content, "## Do the thing\n");
        assert_eq!(bundle.skills.len(), 1);
        assert_eq!(bundle.skills[0].name, "gh");
        assert_eq!(bundle.mcps.len(), 1);
        assert_eq!(bundle.mcps[0].command.as_deref(), Some("sh"));
        assert_eq!(bundle.mcp_mode, McpMode::Only);
        assert_eq!(bundle.launch_args, vec!["--model", "fable"]);
        assert!(bundle.hooks.is_empty());
        assert_eq!(bundle.id, None);
    }

    #[test]
    fn a_local_workflow_file_is_read_with_its_inject_mode() {
        let t = TempDir::new();
        t.write("wf.md", "First invoke /gh.");
        let d = LocalKitDef {
            workflow: Some("./wf.md".into()),
            workflow_inject: Some(WorkflowInject::User),
            ..def()
        };
        let bundle = assemble_local_kit("w", &d, &t.root).unwrap();
        assert_eq!(bundle.workflow.as_deref(), Some("First invoke /gh."));
        assert_eq!(bundle.workflow_inject, WorkflowInject::User);
    }

    #[test]
    fn a_missing_local_prompt_file_errors_with_the_path() {
        let t = TempDir::new();
        let d = LocalKitDef {
            prompt: Some("./nope.md".into()),
            ..def()
        };
        let err = assemble_local_kit("m", &d, &t.root)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("failed to read local prompt file"),
            "got: {err}"
        );
    }

    #[test]
    fn a_part_that_is_not_a_local_path_errors() {
        let t = TempDir::new();
        for bad in ["review.md", "mitchldtn/code:review"] {
            let d = LocalKitDef {
                prompt: Some(bad.to_string()),
                ..def()
            };
            let err = assemble_local_kit("m", &d, &t.root)
                .unwrap_err()
                .to_string();
            assert!(err.contains("must be a local path"), "got: {err}");
        }
    }
}
