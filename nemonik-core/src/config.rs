//! Which nemonik deployment to talk to: `NEMONIK_API`, else `nemonik.io`.
//! Resolved once in `main` and read back via `ApiConfig::current()`.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const API_ENV_VAR: &str = "NEMONIK_API";
pub const DEFAULT_DOMAIN: &str = "nemonik.io";

static CURRENT: OnceLock<ApiConfig> = OnceLock::new();

pub struct ApiConfig {
    domain: String,
}

impl ApiConfig {
    fn resolve() -> Self {
        let domain = std::env::var(API_ENV_VAR)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_DOMAIN.to_string());
        Self { domain }
    }

    /// Resolves and stores the config. Call exactly once, in `main`.
    pub fn init() -> Result<&'static ApiConfig> {
        let config = Self::resolve();
        CURRENT
            .set(config)
            .map_err(|_| anyhow::anyhow!("ApiConfig::init called more than once"))?;
        Ok(CURRENT.get().expect("just set above"))
    }

    /// The config `init` resolved. Panics if `init` hasn't run.
    pub fn current() -> &'static ApiConfig {
        CURRENT
            .get()
            .expect("ApiConfig::init must run in main before this is called")
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn is_default(&self) -> bool {
        self.domain == DEFAULT_DOMAIN
    }

    /// `mcp.<domain>`: both the OAuth audience and the `/resource` fetch base.
    pub fn resource_base_url(&self) -> String {
        format!("https://mcp.{}", self.domain)
    }

    pub fn mcp_endpoint(&self) -> String {
        format!("https://mcp.{}/mcp", self.domain)
    }

    /// Fixed, not derived from the domain: the server requires the fetched document's
    /// `client_id` to equal the URL it fetched, and the hosted document names this URL.
    pub fn cimd_url(&self) -> &'static str {
        "https://nemonik.io/oauth/nemonik-client-metadata.json"
    }

    /// Reverse-DNS, because a bare `nemonik` collides with Claude Code's own keychain
    /// entry.
    pub fn keychain_service(&self) -> String {
        if self.is_default() {
            "io.nemonik.cli".to_string()
        } else {
            format!("io.nemonik.cli.{}", self.domain)
        }
    }
}

/// `~/.nemonik/config.json`. Unknown keys are an error, so a typo isn't silently dropped.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    trust: Option<TrustMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_window_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    herdr: Option<bool>,
}

fn config_path() -> Result<PathBuf> {
    Ok(crate::auth::nemonik_dir()?.join("config.json"))
}

fn read_config_at(path: &Path) -> Result<Config> {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse {}; fix or delete it", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
    }
}

// Temp file + rename, with `0600` set before the rename, so no one can read a
// partial file or another local user flip the gate to `off`.
fn write_config_at(path: &Path, config: &Config) -> Result<()> {
    let tmp_path = path.with_file_name(format!("config.json.tmp.{}", std::process::id()));
    let json = serde_json::to_string_pretty(config)? + "\n";
    std::fs::write(&tmp_path, json)
        .with_context(|| format!("failed to write {}", tmp_path.display()))?;
    if let Err(e) = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600)) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).context("failed to set config file permissions");
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).with_context(|| format!("failed to write {}", path.display()));
    }
    Ok(())
}

fn set_trust_at(path: &Path, mode: TrustMode) -> Result<()> {
    let mut config = read_config_at(path)?;
    config.trust = Some(mode);
    write_config_at(path, &config)
}

/// A blank command counts as unset.
fn window_command(config: &Config) -> Option<String> {
    config
        .new_window_command
        .clone()
        .filter(|s| !s.trim().is_empty())
}

/// The `--new` command template, or an error when unset, so `--new` never silently does
/// nothing.
pub fn require_new_window_command() -> Result<String> {
    window_command(&read_config_at(&config_path()?)?).ok_or_else(|| {
        anyhow::anyhow!(
            "--new needs `new_window_command` in ~/.nemonik/config.json, but it isn't set.\n  \
             Set it to one shell command line with a {{session}} placeholder (examples at \
             https://docs.nemonik.io/config/#window-commands), or drop --new to attach here."
        )
    })
}

/// herdr sets `HERDR_ENV=1` in its panes.
fn inside_herdr(herdr_env: Option<&str>, config: &Config) -> bool {
    herdr_env == Some("1") && config.herdr != Some(false)
}

/// The file is read only inside herdr.
pub fn herdr_active() -> Result<bool> {
    let herdr_env = std::env::var("HERDR_ENV").ok();
    if herdr_env.as_deref() != Some("1") {
        return Ok(false);
    }
    Ok(inside_herdr(
        herdr_env.as_deref(),
        &read_config_at(&config_path()?)?,
    ))
}

// Trust policy: `--trust` > NEMONIK_TRUST > `config.json` > `strict`. Never read
// from `nemonik.json`: a repo must not be able to relax its own gate.

pub const TRUST_ENV_VAR: &str = "NEMONIK_TRUST";

/// The verify-gate policy. There is deliberately no trust-on-first-use mode:
/// auto-trusting a first run is the attack the gate exists to stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    Strict,
    Off,
}

impl TrustMode {
    pub fn as_str(self) -> &'static str {
        match self {
            TrustMode::Strict => "strict",
            TrustMode::Off => "off",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "strict" => Ok(TrustMode::Strict),
            "off" => Ok(TrustMode::Off),
            other => bail!("unknown trust mode '{other}', expected 'strict' or 'off'"),
        }
    }
}

pub fn resolve_trust(flag: Option<&str>) -> Result<TrustMode> {
    let env = std::env::var(TRUST_ENV_VAR).ok();
    // Read only when needed, so a broken file can't block the overrides.
    let file = if flag.is_none() && env.is_none() {
        read_config_at(&config_path()?)?.trust
    } else {
        None
    };
    resolve_trust_with_sources(flag, env.as_deref(), file)
}

// A bad higher-precedence value errors; it never falls through to a weaker source.
fn resolve_trust_with_sources(
    flag: Option<&str>,
    env: Option<&str>,
    file: Option<TrustMode>,
) -> Result<TrustMode> {
    if let Some(flag) = flag {
        return TrustMode::parse(flag);
    }
    if let Some(env) = env {
        return TrustMode::parse(env);
    }
    Ok(file.unwrap_or(TrustMode::Strict))
}

pub fn trust_command(mode: Option<String>) -> Result<()> {
    match mode {
        Some(mode) => {
            let parsed = TrustMode::parse(&mode)?;
            set_trust_at(&config_path()?, parsed)?;
            println!(
                "Trust policy set to '{}' (~/.nemonik/config.json).",
                parsed.as_str()
            );
            if parsed == TrustMode::Off {
                eprintln!(
                    "WARNING: trust is now 'off'; `nemonik run` will launch kits WITHOUT reviewing \
                     them. Re-enable with `nemonik trust strict`."
                );
            }
        }
        None => {
            let resolved = resolve_trust(None)?;
            println!(
                "Trust policy: {} (--trust > {TRUST_ENV_VAR} > ~/.nemonik/config.json > default strict)",
                resolved.as_str()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_config_at_rejects_an_unknown_key_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"new-window-command": "x"}"#).unwrap();
        let err = format!("{:#}", read_config_at(&path).unwrap_err());
        assert!(err.contains("new-window-command"), "{err}");
        assert!(err.contains("config.json"), "{err}");
    }

    #[test]
    fn read_config_at_rejects_a_non_boolean_herdr_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"herdr": "yes"}"#).unwrap();
        let err = format!("{:#}", read_config_at(&path).unwrap_err());
        assert!(err.contains("config.json"), "{err}");
    }

    #[test]
    fn inside_herdr_needs_herdr_env_1_and_no_herdr_false() {
        let with = |herdr| Config {
            herdr,
            ..Config::default()
        };
        assert!(inside_herdr(Some("1"), &with(None)));
        assert!(inside_herdr(Some("1"), &with(Some(true))));
        assert!(!inside_herdr(Some("1"), &with(Some(false))));
        assert!(!inside_herdr(Some("0"), &with(None)));
        assert!(!inside_herdr(None, &with(Some(true))));
    }

    #[test]
    fn window_command_treats_blank_as_unset() {
        let with = |s: &str| Config {
            new_window_command: Some(s.to_string()),
            ..Config::default()
        };
        assert_eq!(window_command(&with(" \n ")), None);
        assert_eq!(window_command(&Config::default()), None);
    }

    #[test]
    fn set_trust_at_keeps_the_window_command_and_writes_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{"new_window_command": "echo {session}", "herdr": false}"#,
        )
        .unwrap();
        set_trust_at(&path, TrustMode::Off).unwrap();
        set_trust_at(&path, TrustMode::Strict).unwrap();
        let config = read_config_at(&path).unwrap();
        assert_eq!(config.trust, Some(TrustMode::Strict));
        assert_eq!(window_command(&config), Some("echo {session}".to_string()));
        assert_eq!(config.herdr, Some(false));
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config file must be owner-only");
    }

    #[test]
    fn trust_mode_parse_rejects_unknown() {
        let err = TrustMode::parse("loose").unwrap_err().to_string();
        assert!(err.contains("loose"));
        assert!(err.contains("strict") && err.contains("off"));
    }

    #[test]
    fn resolve_trust_precedence_flag_over_env_over_file_over_default() {
        assert_eq!(
            resolve_trust_with_sources(Some("off"), Some("strict"), Some(TrustMode::Strict))
                .unwrap(),
            TrustMode::Off
        );
        assert_eq!(
            resolve_trust_with_sources(None, Some("off"), Some(TrustMode::Strict)).unwrap(),
            TrustMode::Off
        );
        assert_eq!(
            resolve_trust_with_sources(None, None, Some(TrustMode::Off)).unwrap(),
            TrustMode::Off
        );
        assert_eq!(
            resolve_trust_with_sources(None, None, None).unwrap(),
            TrustMode::Strict
        );
    }

    #[test]
    fn resolve_trust_rejects_a_bad_source_rather_than_falling_through() {
        assert!(resolve_trust_with_sources(Some("nope"), None, Some(TrustMode::Off)).is_err());
        assert!(resolve_trust_with_sources(None, Some("nope"), Some(TrustMode::Strict)).is_err());
    }
}
