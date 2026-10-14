//! Client for nemonik's server: MCP over Streamable HTTP, plus the `/resource` kit fetch.

use anyhow::{Context, Result};
use http::{HeaderName, HeaderValue};
use rmcp::model::{CallToolRequestParams, ProgressNotificationParam};
use rmcp::service::{NotificationContext, RunningService};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{ClientHandler, ClientLifecycleMode, ClientServiceExt, RoleClient};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

// Cloudflare's bot protection 403s default User-Agents.
pub(crate) const USER_AGENT: &str = concat!(
    "nemonik/",
    env!("CARGO_PKG_VERSION"),
    " (+https://nemonik.io)"
);

/// Connects to nemonik's MCP endpoint. `subscriptions/listen` needs protocol
/// `2026-07-28`, so use `ClientLifecycleMode::Discover` for it, not `Initialize`.
pub async fn connect<H: ClientHandler>(
    lifecycle: ClientLifecycleMode,
    handler: H,
) -> Result<RunningService<RoleClient, H>> {
    let access_token = crate::auth::current_access_token().await?;

    let mut headers = HashMap::new();
    headers.insert(
        HeaderName::from_static("user-agent"),
        HeaderValue::from_static(USER_AGENT),
    );

    let mut config = StreamableHttpClientTransportConfig::default();
    config.uri = crate::config::ApiConfig::current().mcp_endpoint().into();
    config.auth_header = Some(access_token);
    config.custom_headers = headers;
    let transport = StreamableHttpClientTransport::from_config(config);

    handler
        .serve_with_lifecycle(transport, lifecycle)
        .await
        .context("failed to connect to nemonik's MCP server")
}

/// Routes `notifications/progress` (how `remote_watch` streams chunks) into `progress`.
#[derive(Debug, Clone, Default)]
pub struct ProgressAwareHandler {
    pub progress: rmcp::handler::client::progress::ProgressDispatcher,
}

impl ClientHandler for ProgressAwareHandler {
    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.progress.handle_notification(params).await;
    }
}

pub fn tool_params(name: &str, arguments: serde_json::Map<String, Value>) -> CallToolRequestParams {
    let mut params = CallToolRequestParams::default();
    params.name = name.to_string().into();
    params.arguments = Some(arguments);
    params
}

/// Extracts a tool call's JSON: `structured_content`, else the first text block that parses.
pub fn extract_json_result(result: &rmcp::model::CallToolResult, tool_name: &str) -> Result<Value> {
    if let Some(structured) = &result.structured_content {
        return Ok(structured.clone());
    }
    for block in &result.content {
        if let Some(text) = block.as_text()
            && let Ok(value) = serde_json::from_str::<Value>(&text.text)
        {
            return Ok(value);
        }
    }
    anyhow::bail!("{tool_name}() returned no structured content or parseable text")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptSpec {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillSpec {
    pub name: String,
    pub description: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpKind {
    Stdio,
    Http,
}

/// An attached MCP's resolved spec, with `env` already decrypted by the server.
/// `env`/`headers` are `BTreeMap`s so `trust::bundle_hash` is deterministic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: McpKind,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// An attached hook; `config` is a Claude Code hooks-settings fragment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookSpec {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub config: serde_json::Value,
}

/// Whether a kit's MCPs extend the agent's own MCP servers or replace them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum McpMode {
    #[default]
    Extend,
    Only,
}

/// Where a kit's `workflow` is injected: the first user turn, the system prompt, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowInject {
    User,
    System,
    #[default]
    Both,
}

/// A fully-resolved kit: what `trust::bundle_hash` hashes and `launch::run` injects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KitBundle {
    /// The resolved kit's stable id, pinned alongside the content hash.
    #[serde(default)]
    pub id: Option<String>,
    pub prompt: Option<PromptSpec>,
    #[serde(default)]
    pub skills: Vec<SkillSpec>,
    #[serde(default)]
    pub mcps: Vec<McpSpec>,
    #[serde(default)]
    pub mcp_mode: McpMode,
    #[serde(default)]
    pub hooks: Vec<HookSpec>,
    /// Extra flags appended verbatim, in order, to the agent's argv.
    #[serde(default)]
    pub launch_args: Vec<String>,
    /// Workflow prose injected into the agent, per `workflow_inject`.
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub workflow_inject: WorkflowInject,
}

fn resource_url(base: &str, name: &str) -> Result<reqwest::Url> {
    let mut url =
        reqwest::Url::parse(base).with_context(|| format!("invalid resource base url {base:?}"))?;
    url.set_path("/resource");
    url.query_pairs_mut()
        .append_pair("type", "kit")
        .append_pair("name", name);
    Ok(url)
}

fn parse_kit_response(status: reqwest::StatusCode, body: &[u8]) -> Result<KitBundle> {
    if status.is_success() {
        return serde_json::from_slice(body)
            .context("kit /resource response did not match the expected bundle shape");
    }
    let detail = resource_error_detail(body);
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("kit not found (unverified or does not exist): {detail}");
    }
    anyhow::bail!(
        "kit /resource fetch failed (HTTP {}): {detail}",
        status.as_u16()
    )
}

fn resource_error_detail(body: &[u8]) -> String {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| String::from_utf8_lossy(body).into_owned())
}

/// Fetches a kit's resolved bundle by its `owner/category:name` address.
pub async fn fetch_kit_bundle(name: &str) -> Result<KitBundle> {
    let (status, body) = get_resource(name).await?;
    parse_kit_response(status, &body)
}

async fn get_resource(name: &str) -> Result<(reqwest::StatusCode, Vec<u8>)> {
    let access_token = crate::auth::current_access_token().await?;
    let base = crate::config::ApiConfig::current().resource_base_url();
    let url = resource_url(&base, name)?;

    // reqwest has no default timeout; without these a stalled fetch hangs verify/run.
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("building the HTTP client for the resource fetch failed")?;

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .context("kit /resource request failed")?;

    let status = response.status();
    let body = response
        .bytes()
        .await
        .context("reading the kit /resource response body failed")?;
    Ok((status, body.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kit_resource_url_form_encodes_the_owner_qualified_name() {
        let url = resource_url("https://mcp.nemonik.io", "mitchldtn/pr:default").unwrap();
        assert_eq!(
            url.as_str(),
            "https://mcp.nemonik.io/resource?type=kit&name=mitchldtn%2Fpr%3Adefault"
        );
    }

    #[test]
    fn parse_kit_response_maps_404_to_a_not_found_error_surfacing_server_detail() {
        let body = br#"{"error":"kit 'x' not found"}"#;
        let msg = parse_kit_response(reqwest::StatusCode::NOT_FOUND, body)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("not found"), "got: {msg}");
        assert!(
            msg.contains("kit 'x' not found"),
            "server detail should surface: {msg}"
        );
    }

    #[test]
    fn parse_kit_response_bails_on_other_statuses_with_status_and_detail() {
        let body = br#"{"error":"boom"}"#;
        let msg = parse_kit_response(reqwest::StatusCode::INTERNAL_SERVER_ERROR, body)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("500"), "status in message: {msg}");
        assert!(msg.contains("boom"), "detail in message: {msg}");
    }

    #[test]
    fn resource_error_detail_falls_back_to_raw_body_when_not_json() {
        assert_eq!(resource_error_detail(b"plain text oops"), "plain text oops");
    }

    #[test]
    fn kit_bundle_deserializes_attached_skills_and_mcps() {
        let json = serde_json::json!({
            "prompt": null,
            "skills": [{
                "name": "gh",
                "description": "Use the GitHub CLI...",
                "body": "---\nname: gh\n---\n\n# gh\n"
            }],
            "mcps": [{
                "name": "format",
                "type": "stdio",
                "command": "sh",
                "args": ["-c", "echo $TEST_KEY"],
                "url": null,
                "headers": {},
                "env": { "TEST_KEY": "TEST1" }
            }]
        });
        let bundle: KitBundle = serde_json::from_value(json).unwrap();

        assert_eq!(bundle.skills.len(), 1);
        assert_eq!(bundle.skills[0].name, "gh");
        assert_eq!(bundle.skills[0].body, "---\nname: gh\n---\n\n# gh\n");

        assert_eq!(bundle.mcps.len(), 1);
        let mcp = &bundle.mcps[0];
        assert_eq!(mcp.name, "format");
        assert_eq!(mcp.kind, McpKind::Stdio);
        assert_eq!(mcp.command.as_deref(), Some("sh"));
        assert_eq!(
            mcp.args,
            vec!["-c".to_string(), "echo $TEST_KEY".to_string()]
        );
        assert_eq!(mcp.env.get("TEST_KEY").map(String::as_str), Some("TEST1"));
    }
}
