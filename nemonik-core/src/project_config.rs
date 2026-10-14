//! Project-local `nemonik.json`, found by walking up from the cwd. It may come from
//! an untrusted clone, so only `project`, `aliases`, and `kits` are honored, and the
//! trust gate still reviews whatever kit they resolve to.

use crate::mcp::{McpMode, WorkflowInject};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Unknown keys (including `yolo` or any trust setting) are ignored, never honored.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ProjectConfig {
    /// Default project when `--project`/`$NEMONIK_PROJECT` are both absent.
    #[serde(default)]
    pub project: Option<String>,
    /// `alias -> kit address` map. Values are handed to the server verbatim,
    /// so they must be `owner/category:name` addresses.
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    /// Local kits; a name here wins over an alias of the same name.
    #[serde(default)]
    pub kits: HashMap<String, LocalKitDef>,
}

/// A local kit: every part is a local path, resolved against the `nemonik.json` directory.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct LocalKitDef {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default, deserialize_with = "string_or_seq")]
    pub skills: Vec<String>,
    /// Path(s) to JSON arrays of `McpSpec`.
    #[serde(default, deserialize_with = "string_or_seq")]
    pub mcps: Vec<String>,
    /// Path(s) to JSON arrays of `HookSpec`.
    #[serde(default, deserialize_with = "string_or_seq")]
    pub hooks: Vec<String>,
    /// `"extend"` (default) or `"only"`, same meaning as a server kit's.
    #[serde(default)]
    pub mcp_mode: Option<McpMode>,
    /// Extra CLI flags appended verbatim to the launched agent.
    #[serde(default, deserialize_with = "string_or_seq")]
    pub launch_args: Vec<String>,
    /// Path to a `.md` file whose text is the kit's workflow.
    #[serde(default)]
    pub workflow: Option<String>,
    /// `"user"`, `"system"`, or `"both"` (default), same meaning as a server kit's.
    #[serde(default)]
    pub workflow_inject: Option<WorkflowInject>,
}

/// Accepts a single string or an array.
fn string_or_seq<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}

/// True for an explicit local path (`./`, `../`, `/`, `~`, `.`, `..`). Syntactic on
/// purpose: a bare relative path is never guessed into a local file read.
pub fn is_local_path(value: &str) -> bool {
    value == "."
        || value == ".."
        || value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with('/')
        || value.starts_with('~')
}

/// Resolves a local path against the `nemonik.json` directory; `~` and `~/...` expand
/// to `$HOME`.
pub fn resolve_local_path(base: &Path, value: &str) -> PathBuf {
    if value == "~" {
        return home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    let p = Path::new(value);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

fn home_dir() -> PathBuf {
    crate::auth::home_dir().unwrap_or_else(|_| PathBuf::from("~"))
}

const CONFIG_FILE_NAME: &str = "nemonik.json";

/// Walks up from `start` to the first `nemonik.json`, stopping at a `.git` boundary.
fn find_config_file(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let candidate = dir.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        // Checked after the candidate, so a repo-root `nemonik.json` still counts.
        if dir.join(".git").exists() {
            break;
        }
    }
    None
}

/// Malformed JSON is an error, never a silent empty config.
fn parse(contents: &str) -> Result<ProjectConfig> {
    Ok(serde_json::from_str(contents)?)
}

/// The nearest `nemonik.json` and the directory that holds it.
pub fn discover() -> Result<(ProjectConfig, Option<PathBuf>)> {
    let cwd = std::env::current_dir().context("failed to resolve the current working directory")?;
    load_from(&cwd)
}

fn load_from(start: &Path) -> Result<(ProjectConfig, Option<PathBuf>)> {
    let Some(path) = find_config_file(start) else {
        return Ok((ProjectConfig::default(), None));
    };
    let base = path.parent().map(Path::to_path_buf);
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    // A blank file is an empty config: JSON has no empty document.
    if contents.trim().is_empty() {
        return Ok((ProjectConfig::default(), base));
    }
    let config = parse(&contents).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok((config, base))
}

/// `--project` > `$NEMONIK_PROJECT` > `nemonik.json` > none.
pub fn resolve_project(
    flag: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> Option<String> {
    flag.or(env)
        .or(config.project.as_deref())
        .map(str::to_string)
}

pub fn expand_alias<'a>(kit: &'a str, aliases: &'a HashMap<String, String>) -> &'a str {
    aliases.get(kit).map(String::as_str).unwrap_or(kit)
}

/// The effective project and kit, and whether each came from `nemonik.json`.
#[derive(Debug, PartialEq, Eq)]
pub struct Resolved {
    pub project: Option<String>,
    pub kit: String,
    /// The kit address came from an alias in `nemonik.json`.
    pub expanded: bool,
    /// The project came from `nemonik.json`; announced because the file may be untrusted.
    pub project_from_config: bool,
}

pub fn resolve(
    flag_project: Option<&str>,
    env_project: Option<&str>,
    kit: &str,
    config: &ProjectConfig,
) -> Resolved {
    let project = resolve_project(flag_project, env_project, config);
    // Only when neither the flag nor the env supplied it.
    let project_from_config =
        flag_project.is_none() && env_project.is_none() && config.project.is_some();
    let expanded_kit = expand_alias(kit, &config.aliases).to_string();
    let expanded = expanded_kit != kit;
    Resolved {
        project,
        kit: expanded_kit,
        expanded,
        project_from_config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTree {
        _dir: tempfile::TempDir,
        root: PathBuf,
    }

    impl TestTree {
        fn new(name: &str) -> Self {
            let dir = tempfile::Builder::new().prefix(name).tempdir().unwrap();
            let root = dir.path().to_path_buf();
            // A `.git` at the root keeps the walk from escaping into temp_dir's ancestors.
            std::fs::create_dir_all(root.join(".git")).unwrap();
            Self { _dir: dir, root }
        }
        fn mkdir(&self, rel: &str) -> PathBuf {
            let p = self.root.join(rel);
            std::fs::create_dir_all(&p).unwrap();
            p
        }
        fn write(&self, rel: &str, contents: &str) {
            let p = self.root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, contents).unwrap();
        }
    }

    #[test]
    fn find_config_file_walks_up_to_an_ancestor() {
        let t = TestTree::new("ancestor");
        t.write("nemonik.json", "{\"project\": \"x\"}");
        let deep = t.mkdir("a/b/c");
        let found = find_config_file(&deep).unwrap();
        assert_eq!(found, t.root.join("nemonik.json"));
    }

    #[test]
    fn find_config_file_returns_none_when_absent() {
        let t = TestTree::new("absent");
        let deep = t.mkdir("a/b");
        assert_eq!(find_config_file(&deep), None);
    }

    #[test]
    fn find_config_file_stops_at_a_git_boundary() {
        let t = TestTree::new("git-boundary");
        t.write("nemonik.json", "{\"project\": \"outer\"}");
        t.mkdir("repo/.git");
        let inside = t.mkdir("repo/src");
        assert_eq!(find_config_file(&inside), None);
    }

    #[test]
    fn find_config_file_honors_a_config_at_the_repo_root_itself() {
        let t = TestTree::new("repo-root");
        t.mkdir("repo/.git");
        t.write("repo/nemonik.json", "{\"project\": \"inner\"}");
        let inside = t.mkdir("repo/src");
        let found = find_config_file(&inside).unwrap();
        assert_eq!(found, t.root.join("repo/nemonik.json"));
    }

    #[test]
    fn parse_ignores_unknown_and_sensitive_keys() {
        // Unknown keys, including `api`/`yolo`/`trust`, are never honored.
        let cfg = parse(
            "{\"project\": \"x\", \"api\": \"evil.example\", \"yolo\": true, \"trust\": \"off\", \"projet\": \"typo\"}",
        )
        .unwrap();
        assert_eq!(cfg.project.as_deref(), Some("x"));
        assert!(cfg.aliases.is_empty());
    }

    #[test]
    fn load_from_treats_an_empty_file_as_default_not_a_parse_error() {
        let t = TestTree::new("load-empty");
        t.write("nemonik.json", "\n  \n");
        assert_eq!(load_from(&t.root).unwrap().0, ProjectConfig::default());
    }

    #[test]
    fn load_from_names_the_file_it_failed_to_parse() {
        let t = TestTree::new("load-bad");
        let deep = t.mkdir("a/b");
        t.write("nemonik.json", r#"{"kits": {"k": {"mcp_mode": "onyl"}}}"#);
        let err = format!("{:#}", load_from(&deep).unwrap_err());
        let path = t.root.join("nemonik.json");
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(err.contains("unknown variant `onyl`"), "{err}");
    }

    #[test]
    fn resolve_project_prefers_flag_then_env_then_config() {
        let cfg = ProjectConfig {
            project: Some("from-config".into()),
            aliases: HashMap::new(),
            kits: HashMap::new(),
        };
        assert_eq!(
            resolve_project(Some("flag"), Some("env"), &cfg).as_deref(),
            Some("flag")
        );
        assert_eq!(
            resolve_project(None, Some("env"), &cfg).as_deref(),
            Some("env")
        );
        assert_eq!(
            resolve_project(None, None, &cfg).as_deref(),
            Some("from-config")
        );
    }

    fn config_with(project: Option<&str>, aliases: &[(&str, &str)]) -> ProjectConfig {
        ProjectConfig {
            project: project.map(str::to_string),
            aliases: aliases
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            kits: HashMap::new(),
        }
    }

    #[test]
    fn resolve_composes_project_precedence_with_alias_expansion() {
        let cfg = config_with(Some("cfgproj"), &[("review", "mitchldtn/code:review")]);
        assert_eq!(
            resolve(None, None, "review", &cfg),
            Resolved {
                project: Some("cfgproj".into()),
                kit: "mitchldtn/code:review".into(),
                expanded: true,
                project_from_config: true,
            }
        );
        assert_eq!(
            resolve(
                Some("flagproj"),
                Some("envproj"),
                "mitchldtn/code:task",
                &cfg
            ),
            Resolved {
                project: Some("flagproj".into()),
                kit: "mitchldtn/code:task".into(),
                expanded: false,
                project_from_config: false,
            }
        );
        assert_eq!(
            resolve(None, Some("envproj"), "review", &cfg)
                .project
                .as_deref(),
            Some("envproj")
        );
    }

    #[test]
    fn resolve_flags_project_from_config_only_when_config_actually_supplied_it() {
        let cfg = config_with(Some("cfgproj"), &[]);
        assert!(resolve(None, None, "k:x", &cfg).project_from_config);
        assert!(!resolve(Some("cfgproj"), None, "k:x", &cfg).project_from_config);
        assert!(!resolve(None, Some("cfgproj"), "k:x", &cfg).project_from_config);
        assert!(!resolve(None, None, "k:x", &config_with(None, &[])).project_from_config);
    }

    #[test]
    fn is_local_path_classifies_paths_vs_server_addresses() {
        for local in ["./p.md", "../x", "/abs/p.md", "~/p.md", "~", ".", ".."] {
            assert!(is_local_path(local), "{local} should be local");
        }
        for remote in [
            "mitchldtn/code:review",
            "owner/name",
            "code:review",
            "x",
            "team/skill",
        ] {
            assert!(
                !is_local_path(remote),
                "{remote} should be a server address"
            );
        }
    }

    #[test]
    fn resolve_local_path_joins_relative_keeps_absolute_expands_tilde() {
        let base = Path::new("/repo/cfgdir");
        assert_eq!(
            resolve_local_path(base, "./p.md"),
            Path::new("/repo/cfgdir/./p.md")
        );
        assert_eq!(
            resolve_local_path(base, "sub/p.md"),
            Path::new("/repo/cfgdir/sub/p.md")
        );
        assert_eq!(
            resolve_local_path(base, "/abs/p.md"),
            Path::new("/abs/p.md")
        );
        let tilde = resolve_local_path(base, "~/p.md");
        assert!(tilde.ends_with("p.md"));
        assert!(!tilde.starts_with(base));
    }

    #[test]
    fn kits_map_parses_with_string_or_array_and_defaults() {
        let json = r#"{
            "kits": {
                "mine": {
                    "prompt": "./prompts/mine.md",
                    "skills": "./skills",
                    "mcps": ["./mcps.json", "./more-mcps.json"],
                    "launch_args": ["--model", "fable"]
                },
                "bare": {}
            }
        }"#;
        let cfg = parse(json).unwrap();
        let mine = &cfg.kits["mine"];
        assert_eq!(mine.prompt.as_deref(), Some("./prompts/mine.md"));
        assert_eq!(mine.skills, vec!["./skills"]);
        assert_eq!(mine.mcps, vec!["./mcps.json", "./more-mcps.json"]);
        assert_eq!(mine.launch_args, vec!["--model", "fable"]);
        assert!(mine.hooks.is_empty());
        assert_eq!(mine.mcp_mode, None);
        assert_eq!(cfg.kits["bare"], LocalKitDef::default());
    }

    #[test]
    fn an_invalid_mcp_mode_or_workflow_inject_fails_the_parse() {
        // A typo must not silently re-open the agent's own MCPs or drop the workflow.
        let err = parse(r#"{"kits": {"k": {"mcp_mode": "onlyy"}}}"#).unwrap_err();
        assert!(
            format!("{err:#}").contains("unknown variant `onlyy`"),
            "{err:#}"
        );
        let err = parse(r#"{"kits": {"k": {"workflow_inject": "first-turn"}}}"#).unwrap_err();
        assert!(
            format!("{err:#}").contains("unknown variant `first-turn`"),
            "{err:#}"
        );
        let ok = parse(r#"{"kits": {"k": {"mcp_mode": "only"}}}"#).unwrap();
        assert_eq!(ok.kits["k"].mcp_mode, Some(McpMode::Only));
    }

    #[test]
    fn load_from_returns_the_config_dir_as_the_base() {
        let t = TestTree::new("kits-base");
        t.write(
            "nemonik.json",
            "{\"kits\": {\"k\": {\"prompt\": \"./p.md\"}}}",
        );
        let deep = t.mkdir("a/b");
        let (cfg, base) = load_from(&deep).unwrap();
        assert!(cfg.kits.contains_key("k"));
        assert_eq!(base.as_deref(), Some(t.root.as_path()));
    }

    #[test]
    fn load_from_returns_default_and_no_base_when_absent() {
        let t = TestTree::new("kits-nobase");
        let deep = t.mkdir("a/b");
        let (cfg, base) = load_from(&deep).unwrap();
        assert_eq!(cfg, ProjectConfig::default());
        assert_eq!(base, None);
    }
}
