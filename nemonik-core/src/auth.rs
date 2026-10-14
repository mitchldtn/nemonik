//! OAuth login (CIMD + PKCE) and access tokens. `NEMONIK_API_KEY`, when set, is used
//! instead of the OS keychain.

use anyhow::{Context, Result};
use async_trait::async_trait;
use oauth2::TokenResponse;
use rmcp::transport::auth::{
    AuthError, AuthorizationManager, AuthorizationRequest, AuthorizationSession, CredentialStore,
    StoredCredentials,
};
use std::fs::{File, TryLockError};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

const TOKEN_ENV_VAR: &str = "NEMONIK_API_KEY";

/// Forwarded verbatim; the server validates it. Blank counts as unset.
fn token_from_env(lookup: impl Fn(&str) -> Option<String>) -> Option<String> {
    lookup(TOKEN_ENV_VAR).filter(|v| !v.trim().is_empty())
}

/// An env-var token can't be refreshed; a server rejection means the key itself is bad.
fn refresh_guard(lookup: impl Fn(&str) -> Option<String>) -> Result<()> {
    match token_from_env(lookup) {
        Some(_) => anyhow::bail!("{TOKEN_ENV_VAR} was rejected by the server; check the key"),
        None => Ok(()),
    }
}

fn keychain_hint(e: &keyring::Error) -> Option<String> {
    matches!(
        e,
        keyring::Error::NoDefaultStore
            | keyring::Error::PlatformFailure(_)
            | keyring::Error::NoStorageAccess(_)
    )
    .then(|| {
        format!(
            "No usable OS keychain. On a machine without one (headless, SSH, CI), \
             set {TOKEN_ENV_VAR} to your nemonik API key."
        )
    })
}

fn keychain_error(action: &str, e: keyring::Error) -> String {
    match keychain_hint(&e) {
        Some(hint) => format!("{action} failed: {e}\n{hint}"),
        None => format!("{action} failed: {e}"),
    }
}

fn keychain_open_error(e: keyring::Error) -> anyhow::Error {
    let context = match keychain_hint(&e) {
        Some(hint) => format!("opening the OS keychain entry failed\n{hint}"),
        None => "opening the OS keychain entry failed".to_string(),
    };
    anyhow::Error::new(e).context(context)
}

pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("HOME not set"))
}

/// `~/.nemonik/`, created on demand.
pub fn nemonik_dir() -> Result<PathBuf> {
    let mut dir = home_dir()?;
    dir.push(".nemonik");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn auth_lock_path() -> Result<PathBuf> {
    let mut dir = nemonik_dir()?;
    dir.push("auth.lock");
    Ok(dir)
}

const AUTH_LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const AUTH_LOCK_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Serializes token refreshes across `nemonik` processes: concurrent redemptions of
/// the shared refresh token race, and the loser looks logged out. `flock` frees on crash.
async fn acquire_auth_lock() -> Result<File> {
    let path = auth_lock_path()?;
    acquire_lock_at(&path, AUTH_LOCK_TIMEOUT).await
}

async fn acquire_lock_at(path: &std::path::Path, timeout: Duration) -> Result<File> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .with_context(|| format!("failed to open auth lock file at {}", path.display()))?;

    let start = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) => {
                if start.elapsed() > timeout {
                    anyhow::bail!(
                        "timed out waiting for another nemonik process to finish refreshing \
                         the auth token (lock held at {})",
                        path.display()
                    );
                }
                tokio::time::sleep(AUTH_LOCK_POLL_INTERVAL).await;
            }
            Err(TryLockError::Error(e)) => return Err(e).context("failed to acquire auth lock"),
        }
    }
}

/// Genuinely logged out (not a transient failure), so callers can stop retrying.
#[derive(Debug)]
pub struct NotLoggedIn;

impl std::fmt::Display for NotLoggedIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "not logged in: run `nemonik login` (or set {TOKEN_ENV_VAR})"
        )
    }
}

impl std::error::Error for NotLoggedIn {}

struct KeyringCredentialStore {
    entry: keyring::Entry,
}

impl KeyringCredentialStore {
    fn new(service: &str) -> Result<Self> {
        let username = std::env::var("USER").unwrap_or_else(|_| "nemonik-user".to_string());
        let entry = keyring::Entry::new(service, &username).map_err(keychain_open_error)?;
        Ok(Self { entry })
    }
}

#[async_trait]
impl CredentialStore for KeyringCredentialStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, AuthError> {
        match self.entry.get_password() {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| AuthError::InternalError(format!("corrupt stored credentials: {e}"))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AuthError::InternalError(keychain_error("keychain read", e))),
        }
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), AuthError> {
        let raw = serde_json::to_string(&credentials).map_err(|e| {
            AuthError::InternalError(format!("failed to serialize credentials: {e}"))
        })?;
        self.entry
            .set_password(&raw)
            .map_err(|e| AuthError::InternalError(keychain_error("keychain write", e)))
    }

    async fn clear(&self) -> Result<(), AuthError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AuthError::InternalError(keychain_error(
                "keychain delete",
                e,
            ))),
        }
    }
}

fn keyring_store() -> Result<KeyringCredentialStore> {
    KeyringCredentialStore::new(&crate::config::ApiConfig::current().keychain_service())
}

async fn configured_manager() -> Result<AuthorizationManager> {
    let api = crate::config::ApiConfig::current();
    let mut manager = AuthorizationManager::new(api.resource_base_url())
        .await
        .context("failed to construct authorization manager")?;
    manager.set_credential_store(keyring_store()?);
    let resolution = manager
        .resolve_metadata()
        .await
        .context("failed to discover nemonik-api's authorization server metadata")?;
    manager.set_metadata(resolution.metadata);
    // Needed by `get_access_token`'s refresh; only `login` configures it otherwise.
    manager
        .configure_client_id(api.cimd_url())
        .map_err(|e| anyhow::anyhow!("failed to configure OAuth client: {e}"))?;
    Ok(manager)
}

/// Browser-based CIMD + PKCE login on an ephemeral loopback port (RFC 8252).
pub async fn login() -> Result<()> {
    let manager = configured_manager().await?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("failed to bind a local port for the OAuth redirect")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let request = AuthorizationRequest::new(redirect_uri.clone())
        .with_client_metadata_url(crate::config::ApiConfig::current().cimd_url())
        .with_client_name("nemonik");

    let session = AuthorizationSession::new(manager, request)
        .await
        .map_err(|(_manager, e)| e)
        .context("failed to start authorization session")?;

    println!("Opening browser to log in to nemonik...");
    println!(
        "If it doesn't open automatically, visit:\n  {}",
        session.get_authorization_url()
    );
    open_browser(session.get_authorization_url());

    let callback_url = accept_one_redirect(listener, port).await?;
    session
        .handle_callback_url(&callback_url)
        .await
        .context("failed to exchange authorization code for a token")?;

    println!("Logged in.");
    Ok(())
}

async fn accept_one_redirect(listener: TcpListener, port: u16) -> Result<String> {
    let (stream, _) = listener
        .accept()
        .await
        .context("failed waiting for the OAuth redirect")?;
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;
    // "GET /callback?code=...&state=... HTTP/1.1"
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("malformed redirect request: {request_line:?}"))?;

    let response_body = "<html><body>Logged in to nemonik. You can close this tab.</body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    reader.into_inner().write_all(response.as_bytes()).await?;

    Ok(format!("http://127.0.0.1:{port}{path}"))
}

pub fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).status();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).status();
}

/// Fast-path freshness floor. Must stay >= rmcp's private `REFRESH_BUFFER_SECS` (30),
/// or the fast path could return a token rmcp would have refreshed.
const FAST_PATH_MIN_REMAINING_SECS: u64 = 60;

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn cached_token_is_fresh(expires_in_secs: Option<u64>, received_at: Option<u64>, now: u64) -> bool {
    match (expires_in_secs, received_at) {
        (Some(expires_in), Some(received_at)) => {
            let elapsed = now.saturating_sub(received_at);
            expires_in.saturating_sub(elapsed) >= FAST_PATH_MIN_REMAINING_SECS
        }
        _ => true,
    }
}

fn fresh_token_from(stored: &StoredCredentials, now: u64) -> Option<String> {
    let token_response = stored.token_response.as_ref()?;
    let expires_in = token_response.expires_in().map(|d| d.as_secs());
    cached_token_is_fresh(expires_in, stored.token_received_at, now)
        .then(|| token_response.access_token().secret().to_string())
}

/// A cached token with enough life left, skipping OAuth discovery. Lock-free is safe:
/// siblings only refresh below `REFRESH_BUFFER_SECS`, which is under this floor.
async fn try_cached_access_token() -> Option<String> {
    let stored = keyring_store().ok()?.load().await.ok().flatten()?;
    fresh_token_from(&stored, now_epoch_secs())
}

/// A valid access token: `NEMONIK_API_KEY` first, else a cached keychain token, else
/// a locked refresh.
pub async fn current_access_token() -> Result<String> {
    if let Some(token) = token_from_env(|k| std::env::var(k).ok()) {
        return Ok(token);
    }

    if let Some(token) = try_cached_access_token().await {
        return Ok(token);
    }

    // Held across read-or-refresh so a loser re-reads the winner's fresh token.
    let _lock = acquire_auth_lock().await?;

    let manager = configured_manager().await?;
    match manager.get_access_token().await {
        Ok(token) => Ok(token),
        Err(AuthError::AuthorizationRequired) => Err(NotLoggedIn.into()),
        Err(e) => Err(e).context("failed to get access token"),
    }
}

/// Forces a refresh, for when the server rejected a token our clock still considers
/// fresh (`get_access_token` would hand back the same dead token).
pub async fn force_refresh_access_token() -> Result<String> {
    refresh_guard(|k| std::env::var(k).ok())?;
    let _lock = acquire_auth_lock().await?;

    let manager = configured_manager().await?;
    match manager.refresh_token().await {
        Ok(token_response) => Ok(token_response.access_token().secret().to_string()),
        Err(AuthError::AuthorizationRequired | AuthError::TokenRefreshRejected(_)) => {
            Err(NotLoggedIn.into())
        }
        Err(e) => Err(e).context("failed to refresh access token"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_logged_in_message_points_at_login_and_the_env_var_fallback() {
        let message = NotLoggedIn.to_string();
        assert!(message.contains("nemonik login"));
        assert!(message.contains("NEMONIK_API_KEY"));
    }

    fn env_with(key: &'static str, value: &'static str) -> impl Fn(&str) -> Option<String> {
        move |k| (k == key).then(|| value.to_string())
    }

    #[test]
    fn token_from_env_treats_unset_empty_and_blank_as_no_key() {
        assert_eq!(token_from_env(|_| None), None);
        assert_eq!(token_from_env(env_with("OTHER_VAR", "nmk_abc")), None);
        assert_eq!(token_from_env(env_with("NEMONIK_API_KEY", "")), None);
        assert_eq!(token_from_env(env_with("NEMONIK_API_KEY", "  \t")), None);
    }

    #[test]
    fn keychain_error_hints_the_api_key_when_no_store_exists() {
        let message = keychain_error("keychain read", keyring::Error::NoDefaultStore);
        assert!(message.starts_with("keychain read failed"));
        assert!(message.contains("NEMONIK_API_KEY"));
    }

    #[test]
    fn keychain_error_has_no_hint_for_non_availability_failures() {
        for e in [
            keyring::Error::NoEntry,
            keyring::Error::BadEncoding(vec![0xff]),
        ] {
            let message = keychain_error("keychain read", e);
            assert!(!message.contains("NEMONIK_API_KEY"), "{message}");
        }
    }

    #[test]
    fn refresh_is_refused_when_the_rejected_token_came_from_the_env() {
        let message = refresh_guard(env_with("NEMONIK_API_KEY", "nmk_abc"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("NEMONIK_API_KEY"), "{message}");
        assert!(message.contains("rejected"), "{message}");
    }

    #[test]
    fn cached_token_freshness_boundary_is_inclusive_at_the_floor() {
        // 60s left is fresh; 59s is not.
        assert!(cached_token_is_fresh(Some(3600), Some(1000), 1000 + 3540));
        assert!(!cached_token_is_fresh(Some(3600), Some(1000), 1000 + 3541));
    }

    #[test]
    fn cached_token_without_expiry_info_is_returned_as_is() {
        assert!(cached_token_is_fresh(None, Some(1000), 9999));
        assert!(cached_token_is_fresh(Some(3600), None, 9999));
        assert!(cached_token_is_fresh(None, None, 9999));
    }

    fn stored_with(
        access_token: &str,
        expires_in_secs: Option<u64>,
        received_at: Option<u64>,
    ) -> StoredCredentials {
        use oauth2::{AccessToken, basic::BasicTokenType};
        use rmcp::transport::auth::{OAuthTokenResponse, VendorExtraTokenFields};
        let mut resp = OAuthTokenResponse::new(
            AccessToken::new(access_token.to_string()),
            BasicTokenType::Bearer,
            VendorExtraTokenFields::default(),
        );
        if let Some(secs) = expires_in_secs {
            resp.set_expires_in(Some(&Duration::from_secs(secs)));
        }
        StoredCredentials::new("test-client".to_string(), Some(resp), vec![], received_at)
    }

    #[test]
    fn fresh_token_from_uses_received_at_to_age_the_token() {
        let now = 1_000_000;
        let at_floor = stored_with("s", Some(3600), Some(now - (3600 - 60))); // 60s left
        assert_eq!(fresh_token_from(&at_floor, now), Some("s".to_string()));
        let below_floor = stored_with("s", Some(3600), Some(now - (3600 - 59))); // 59s left
        assert_eq!(fresh_token_from(&below_floor, now), None);
    }

    struct TestLockPath {
        _dir: tempfile::TempDir,
        path: PathBuf,
    }

    impl TestLockPath {
        fn new(name: &str) -> Self {
            let dir = tempfile::tempdir().unwrap();
            Self {
                path: dir.path().join(name),
                _dir: dir,
            }
        }
    }

    #[tokio::test]
    async fn acquire_lock_at_times_out_while_another_holder_keeps_it() {
        let lock_path = TestLockPath::new("contended.lock");

        let _held = acquire_lock_at(&lock_path.path, Duration::from_millis(200))
            .await
            .unwrap();

        let second_attempt = acquire_lock_at(&lock_path.path, Duration::from_millis(150)).await;
        assert!(second_attempt.is_err());
    }
}
