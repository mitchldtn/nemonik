//! The private `0600` file the tmux session's `nemonik __exec-agent` reads. A file, not
//! argv: the prompt overflows tmux's command limit and MCP secrets would show in `ps`.

use crate::mcp::SkillSpec;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

/// Everything a session launch and its `__exec-agent` child need.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LaunchSpec {
    pub program: String,
    pub args: Vec<String>,
    pub session: String,
    pub template: String,
    pub skills: Vec<SkillSpec>,
    /// Env set on the agent process (`CODEX_HOME` for Codex); empty for Claude.
    pub env: std::collections::BTreeMap<String, String>,
    pub skills_layout: crate::skills::SkillLayout,
}

pub fn launch_spec_path(session: &str) -> PathBuf {
    std::env::temp_dir().join(format!("nemonik-launch-{session}.json"))
}

/// Writes `spec` to a file created as `0600`, so it is never briefly world-readable.
pub fn write_launch_spec(spec: &LaunchSpec) -> Result<PathBuf> {
    let path = launch_spec_path(&spec.session);
    let json = serde_json::to_string(spec).context("failed to serialize launch spec")?;
    // O_EXCL: refuse a pre-planted symlink; a collision means a session-name bug.
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .with_context(|| format!("failed to create launch spec at {}", path.display()))?;
    f.write_all(json.as_bytes())
        .context("failed to write launch spec")?;
    Ok(path)
}

/// Reads, unlinks, then parses. The unlink must stay before any other fallible
/// work: it bounds how long the prompt and MCP secrets sit on disk.
pub fn consume_launch_spec(spec_path: &str) -> Result<LaunchSpec> {
    let raw = std::fs::read_to_string(spec_path)
        .with_context(|| format!("failed to read launch spec at {spec_path}"))?;
    let _ = std::fs::remove_file(spec_path);
    serde_json::from_str(&raw).context("failed to parse launch spec")
}

/// Best-effort removal, for when `__exec-agent` never consumed the spec.
pub fn remove_launch_spec(session: &str) {
    let _ = std::fs::remove_file(launch_spec_path(session));
}

#[cfg(test)]
mod tests {
    use super::*;

    // `tag` keeps each test's `create_new` path distinct under parallel runs.
    fn spec(tag: &str, program: &str, args: Vec<String>) -> LaunchSpec {
        LaunchSpec {
            program: program.to_string(),
            args,
            session: format!("nemonik-test-{}-{tag}", std::process::id()),
            template: "code:review".to_string(),
            skills: vec![],
            env: Default::default(),
            skills_layout: Default::default(),
        }
    }

    #[test]
    fn write_launch_spec_writes_a_parseable_private_file() {
        let s = spec(
            "write",
            "claude",
            vec!["--verbose".to_string(), "hello\nworld".to_string()],
        );
        remove_launch_spec(&s.session);
        let path = write_launch_spec(&s).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let back: LaunchSpec = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.program, s.program);
        assert_eq!(back.args, s.args);

        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "launch spec must be private: it can hold a prompt and MCP secrets"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn consume_launch_spec_returns_the_spec_and_deletes_the_file() {
        let s = spec(
            "consume",
            "claude",
            vec!["--verbose".to_string(), "big\nprompt".to_string()],
        );
        remove_launch_spec(&s.session);
        let path = write_launch_spec(&s).unwrap();
        assert!(path.exists());

        let back = consume_launch_spec(path.to_str().unwrap()).unwrap();
        assert_eq!(back.program, "claude");
        assert_eq!(
            back.args,
            vec!["--verbose".to_string(), "big\nprompt".to_string()]
        );
        assert!(!path.exists(), "spec must be unlinked on read, before exec");
    }
}
