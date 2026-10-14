//! The harness daemon: relays nemonik's remote-control requests to a session's
//! tmux pane. `harness_fetch_pending` items are `{request_id, kind, command}`; for a
//! capture, `command` is `null` (not absent) or a stringified scrollback line count.

use super::control_mode;
use anyhow::{Context, Result};
use nemonik_core::tmux::{tmux_capture_pane, tmux_has_session_blocking, tmux_submit};
use rmcp::model::{ProtocolVersion, ServerNotification, SubscriptionFilter};
use rmcp::service::{ClientInitializeError, Peer, SubscriptionEnd};
use rmcp::transport::DynamicTransportError;
use rmcp::transport::streamable_http_client::AuthRequiredError;
use rmcp::{ClientLifecycleMode, RoleClient, ServiceError};
use serde::Deserialize;
use serde_json::Value;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// One pending item. `WatchStart`/`WatchStop` are fire-and-forget: no result is reported.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PendingItem {
    Execute {
        request_id: String,
        command: String,
    },
    /// `command` as an optional line count; `u64` rejects a negative value at parse time.
    Capture {
        request_id: String,
        #[serde(rename = "command", default, deserialize_with = "line_count")]
        scrollback_lines: Option<u64>,
    },
    WatchStart {
        request_id: String,
    },
    WatchStop {
        request_id: String,
    },
}

fn extract_requests(raw: &Value) -> Result<Vec<Value>> {
    raw.get("requests")
        .and_then(|r| r.as_array())
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("harness_fetch_pending response missing a \"requests\" array")
        })
}

/// Must match the server's `harness_wake_uri` byte for byte, or wakes never match
/// (pinned by `harness_wake_uri_format_matches_the_server_contract`).
fn harness_wake_uri(project_id: i64, session: &str) -> String {
    format!("harness://{project_id}/{session}")
}

fn extract_project_id(raw: &Value) -> Result<i64> {
    raw.get("project_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            anyhow::anyhow!("harness_register response missing an integer \"project_id\"")
        })
}

fn line_count<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<u64>, D::Error> {
    Option::<String>::deserialize(d)?
        .map(|s| s.parse().map_err(serde::de::Error::custom))
        .transpose()
}

/// Parses one pending item; a bad item is an `Err` for that item only, never a panic.
fn parse_pending_item(raw: &Value) -> Result<PendingItem> {
    let request_id = raw.get("request_id").and_then(Value::as_str).unwrap_or("?");
    PendingItem::deserialize(raw).with_context(|| format!("pending item {request_id}"))
}

/// Exponential backoff: ~500ms doubling to a 30s cap, minus up to 25% jitter.
fn backoff_delay(attempt: u32, jitter_seed: u32) -> Duration {
    let base_ms = 500u64.saturating_mul(1u64 << attempt.min(6));
    let capped_ms = base_ms.min(30_000);
    let max_jitter_ms = capped_ms / 4;
    let jitter_ms = if max_jitter_ms == 0 {
        0
    } else {
        u64::from(jitter_seed) % max_jitter_ms
    };
    Duration::from_millis(capped_ms.saturating_sub(jitter_ms).max(1))
}

fn jitter_seed_from_clock() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0)
}

const AUDIT_LOG_MAX_COMMAND_CHARS: usize = 2000;

/// Escapes control characters so a pushed command can't forge extra log lines (CWE-117).
fn sanitize_for_audit_log(command: &str) -> String {
    let mut out = String::with_capacity(command.len());
    for c in command.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// `<unix-seconds> <request_id> <command>`, truncated on a char boundary, then sanitized.
fn audit_log_line(request_id: &str, command: &str, now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let truncated = truncate_at_char_boundary(command, AUDIT_LOG_MAX_COMMAND_CHARS);
    let sanitized = sanitize_for_audit_log(truncated);
    let mut line = String::new();
    let _ = write!(line, "{secs} {request_id} {sanitized}");
    line
}

fn truncate_at_char_boundary(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

fn result_for_execute() -> Value {
    serde_json::json!({ "delivered": true })
}

fn result_for_capture(pane_text: &str) -> Value {
    serde_json::json!({ "pane": pane_text })
}

/// `spawn_blocking`, since this runs on every liveness tick inside the `select!` loop.
async fn tmux_has_session(session: &str) -> bool {
    let session = session.to_string();
    tokio::task::spawn_blocking(move || tmux_has_session_blocking(&session))
        .await
        .unwrap_or(false)
}

fn audit_log_path() -> Result<PathBuf> {
    let mut dir = nemonik_core::auth::nemonik_dir()?;
    dir.push("harness-daemon.log");
    Ok(dir)
}

/// Best-effort audit log, created as `0600`. Defense in depth, not a gate.
fn append_audit_log(path: &Path, line: &str) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Registers and returns the resolved `project_id`. Wakes are keyed on the id because a
/// project has two spellings (bare for its owner, owner-qualified for a collaborator).
async fn harness_register(peer: &Peer<RoleClient>, project: &str, session: &str) -> Result<i64> {
    let mut args = serde_json::Map::new();
    args.insert("project".into(), project.into());
    args.insert("session".into(), session.into());
    let result = peer
        .call_tool(nemonik_core::mcp::tool_params("harness_register", args))
        .await
        .context("harness_register failed")?;
    let value = nemonik_core::mcp::extract_json_result(&result, "harness_register")?;
    extract_project_id(&value)
}

async fn harness_fetch_pending(
    peer: &Peer<RoleClient>,
    project: &str,
    session: &str,
) -> Result<Vec<Value>> {
    let mut args = serde_json::Map::new();
    args.insert("project".into(), project.into());
    args.insert("session".into(), session.into());
    let result = peer
        .call_tool(nemonik_core::mcp::tool_params(
            "harness_fetch_pending",
            args,
        ))
        .await
        .context("harness_fetch_pending failed")?;
    let value = nemonik_core::mcp::extract_json_result(&result, "harness_fetch_pending")?;
    extract_requests(&value)
}

async fn harness_report_result(
    peer: &Peer<RoleClient>,
    project: &str,
    session: &str,
    request_id: &str,
    result_value: Value,
) -> Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("project".into(), project.into());
    args.insert("session".into(), session.into());
    args.insert("request_id".into(), request_id.into());
    args.insert("result".into(), result_value);
    peer.call_tool(nemonik_core::mcp::tool_params(
        "harness_report_result",
        args,
    ))
    .await
    .context("harness_report_result failed")?;
    Ok(())
}

async fn harness_push_watch_chunk(
    peer: &Peer<RoleClient>,
    project: &str,
    session: &str,
    chunk: String,
) -> Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("project".into(), project.into());
    args.insert("session".into(), session.into());
    args.insert("chunk".into(), chunk.into());
    peer.call_tool(nemonik_core::mcp::tool_params(
        "harness_push_watch_chunk",
        args,
    ))
    .await
    .context("harness_push_watch_chunk failed")?;
    Ok(())
}

async fn harness_end_watch(peer: &Peer<RoleClient>, project: &str, session: &str) -> Result<()> {
    let mut args = serde_json::Map::new();
    args.insert("project".into(), project.into());
    args.insert("session".into(), session.into());
    peer.call_tool(nemonik_core::mcp::tool_params("harness_end_watch", args))
        .await
        .context("harness_end_watch failed")?;
    Ok(())
}

// Watch streaming: `%output` events are debounced into larger chunks, since every
// harness call is rate-limited server-side.

/// How long `run_watch_pump` accumulates live `%output` text into one buffer
/// before pushing it as a single `harness_push_watch_chunk` call.
const WATCH_DEBOUNCE_WINDOW: Duration = Duration::from_millis(100);

/// Early-flush threshold in bytes, so a burst can't grow one chunk unbounded.
const WATCH_MAX_BUFFER_BYTES: usize = 8_000;

/// One thing `run_watch_pump` decided to do with what's currently buffered.
#[derive(Debug, Clone, PartialEq)]
enum WatchChunk {
    Data(String),
    Ended,
    Failed(String),
}

/// One iteration's input to `pump_step`, including timeout and disconnect.
#[derive(Debug, Clone, PartialEq)]
enum PumpInput {
    Event(control_mode::StreamEvent),
    Tick,
    Disconnected,
}

fn is_terminal_input(input: &PumpInput) -> bool {
    matches!(
        input,
        PumpInput::Disconnected
            | PumpInput::Event(control_mode::StreamEvent::Ended)
            | PumpInput::Event(control_mode::StreamEvent::Failed(_))
    )
}

fn flush_buffer(buffer: &mut String, actions: &mut Vec<WatchChunk>) {
    if !buffer.is_empty() {
        actions.push(WatchChunk::Data(std::mem::take(buffer)));
    }
}

/// The pure debounce step: the updated buffer and the chunks to push now.
fn pump_step(mut buffer: String, input: PumpInput) -> (String, Vec<WatchChunk>) {
    let mut actions = Vec::new();
    match input {
        PumpInput::Event(control_mode::StreamEvent::InitialSnapshot(lines)) => {
            // Flush first, to keep arrival order.
            flush_buffer(&mut buffer, &mut actions);
            actions.push(WatchChunk::Data(lines.join("\n")));
        }
        PumpInput::Event(control_mode::StreamEvent::Output { text, .. }) => {
            buffer.push_str(&text);
            // Bytes, not chars: counting chars would be O(n) per event.
            if buffer.len() >= WATCH_MAX_BUFFER_BYTES {
                flush_buffer(&mut buffer, &mut actions);
            }
        }
        PumpInput::Event(control_mode::StreamEvent::Ended) => {
            flush_buffer(&mut buffer, &mut actions);
            actions.push(WatchChunk::Ended);
        }
        PumpInput::Event(control_mode::StreamEvent::Failed(message)) => {
            flush_buffer(&mut buffer, &mut actions);
            actions.push(WatchChunk::Failed(message));
        }
        PumpInput::Tick => {
            flush_buffer(&mut buffer, &mut actions);
        }
        PumpInput::Disconnected => {
            flush_buffer(&mut buffer, &mut actions);
            actions.push(WatchChunk::Ended);
        }
    }
    (buffer, actions)
}

/// Thread body: turns `stream_session` events into debounced chunks; `recv_timeout` is
/// the tick.
fn run_watch_pump(
    rx: std::sync::mpsc::Receiver<control_mode::StreamEvent>,
    tx: tokio::sync::mpsc::UnboundedSender<WatchChunk>,
) {
    let mut buffer = String::new();
    loop {
        let input = match rx.recv_timeout(WATCH_DEBOUNCE_WINDOW) {
            Ok(event) => PumpInput::Event(event),
            Err(RecvTimeoutError::Timeout) => PumpInput::Tick,
            Err(RecvTimeoutError::Disconnected) => PumpInput::Disconnected,
        };
        let is_terminal = is_terminal_input(&input);
        let (next_buffer, actions) = pump_step(buffer, input);
        buffer = next_buffer;
        for action in actions {
            if tx.send(action).is_err() {
                return;
            }
        }
        if is_terminal {
            return;
        }
    }
}

/// Tells the server a watch failed (pushing the reason first), so a `remote_watch`
/// caller, which has no timeout, is unblocked instead of hanging.
async fn fail_watch(peer: &Peer<RoleClient>, project: &str, session: &str, reason: &str) {
    eprintln!("harness daemon: watch stream for session '{session}' failed: {reason}");
    if let Err(e) = harness_push_watch_chunk(
        peer,
        project,
        session,
        format!("[watch failed: {reason}]\n"),
    )
    .await
    {
        eprintln!(
            "harness daemon: harness_push_watch_chunk (failure diagnostic) failed for session '{session}': {e:#}"
        );
    }
    if let Err(e) = harness_end_watch(peer, project, session).await {
        eprintln!("harness daemon: harness_end_watch failed for session '{session}': {e:#}");
    }
}

/// Pushes debounced chunks to the server in order, for one watch attach.
async fn forward_watch_chunks(
    peer: Peer<RoleClient>,
    project: String,
    session: String,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<WatchChunk>,
) {
    while let Some(chunk) = rx.recv().await {
        match chunk {
            WatchChunk::Data(text) => {
                if let Err(e) = harness_push_watch_chunk(&peer, &project, &session, text).await {
                    eprintln!(
                        "harness daemon: harness_push_watch_chunk failed for session '{session}': {e:#}"
                    );
                }
            }
            WatchChunk::Ended => {
                if let Err(e) = harness_end_watch(&peer, &project, &session).await {
                    eprintln!(
                        "harness daemon: harness_end_watch failed for session '{session}': {e:#}"
                    );
                }
                return;
            }
            WatchChunk::Failed(message) => {
                fail_watch(&peer, &project, &session, &message).await;
                return;
            }
        }
    }
}

/// The local watch attach, refcounted: the server sends exactly one `watch_start` and one
/// `watch_stop` per watcher, so overlapping watchers net out.
#[derive(Default)]
struct WatchState {
    handle: Option<control_mode::StreamHandle>,
    chunk_task: Option<tokio::task::JoinHandle<()>>,
    refcount: u32,
}

impl WatchState {
    /// Stops the attach and forwarding task. Aborting can drop a final chunk, which is
    /// fine: this only runs once no `remote_watch` caller is left.
    fn stop_stream(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.stop();
        }
        if let Some(task) = self.chunk_task.take() {
            task.abort();
        }
    }
}

/// Whether the forwarding task is still running; lets `note_watch_start` restart an
/// attach that died without a `watch_stop`.
fn watch_stream_alive(watch: &WatchState) -> bool {
    watch.chunk_task.as_ref().is_some_and(|t| !t.is_finished())
}

async fn note_watch_start(
    watch: &mut WatchState,
    peer: Peer<RoleClient>,
    project: String,
    session: String,
) {
    watch.refcount = watch.refcount.saturating_add(1);
    if watch_stream_alive(watch) {
        return; // already streaming, this watcher rides the existing attach
    }
    watch.stop_stream(); // self-heal: clear out a stale handle/task first, if any

    match control_mode::stream_session(&session) {
        Ok((handle, rx)) => {
            watch.handle = Some(handle);
            let (tx, chunk_rx) = tokio::sync::mpsc::unbounded_channel();
            thread::spawn(move || run_watch_pump(rx, tx));
            watch.chunk_task = Some(tokio::spawn(forward_watch_chunks(
                peer, project, session, chunk_rx,
            )));
        }
        Err(e) => {
            // Tell the server, or a waiting `remote_watch` hangs. The refcount stays
            // until the matching `watch_stop`.
            fail_watch(&peer, &project, &session, &format!("{e:#}")).await;
        }
    }
}

fn note_watch_stop(watch: &mut WatchState) {
    watch.refcount = watch.refcount.saturating_sub(1);
    if watch.refcount == 0 {
        watch.stop_stream();
    }
}

/// Executes one pending item and reports the result; failures are logged, never returned.
async fn dispatch_one(
    peer: &Peer<RoleClient>,
    project: &str,
    session: &str,
    raw: &Value,
    audit_path: &Path,
    watch: &mut WatchState,
) {
    let item = match parse_pending_item(raw) {
        Ok(item) => item,
        Err(e) => {
            eprintln!("harness daemon: skipping unparseable pending item: {e:#}");
            return;
        }
    };

    let (request_id, result_value) = match item {
        PendingItem::WatchStart { .. } => {
            note_watch_start(
                watch,
                peer.clone(),
                project.to_string(),
                session.to_string(),
            )
            .await;
            return;
        }
        PendingItem::WatchStop { .. } => {
            note_watch_stop(watch);
            return;
        }
        PendingItem::Execute {
            request_id,
            command,
        } => match tmux_submit(session, &command) {
            Ok(()) => {
                // Logged only once delivered.
                if let Err(e) = append_audit_log(
                    audit_path,
                    &audit_log_line(&request_id, &command, SystemTime::now()),
                ) {
                    eprintln!("harness daemon: audit log write failed (continuing anyway): {e:#}");
                }
                (request_id, result_for_execute())
            }
            Err(e) => {
                eprintln!("harness daemon: submit failed for {request_id}: {e:#}");
                return;
            }
        },
        PendingItem::Capture {
            request_id,
            scrollback_lines,
        } => match tmux_capture_pane(session, scrollback_lines) {
            Ok(pane) => (request_id, result_for_capture(&pane)),
            Err(e) => {
                eprintln!("harness daemon: capture-pane failed for {request_id}: {e:#}");
                return;
            }
        },
    };

    if let Err(e) = harness_report_result(peer, project, session, &request_id, result_value).await {
        eprintln!("harness daemon: report_result failed for {request_id}: {e:#}");
    }
}

/// Liveness check on the target session while idle; without it an idle daemon outlives
/// its session.
const LIVENESS_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Per-call deadline for MCP round trips: rmcp's HTTP client sets no timeout, so a stuck
/// connect would hang the daemon. Bounds each call, not a whole drain or listen.
const NETWORK_CALL_DEADLINE: Duration = Duration::from_secs(45);

/// `dispatch_one` under the deadline. Only its async `harness_report_result` is bounded;
/// the blocking tmux calls before it can't be interrupted.
async fn dispatch_one_bounded(
    peer: &Peer<RoleClient>,
    project: &str,
    session: &str,
    raw: &Value,
    audit_path: &Path,
    watch: &mut WatchState,
) {
    if tokio::time::timeout(
        NETWORK_CALL_DEADLINE,
        dispatch_one(peer, project, session, raw, audit_path, watch),
    )
    .await
    .is_err()
    {
        eprintln!(
            "harness daemon: dispatch timed out after {NETWORK_CALL_DEADLINE:?} for one pending item, skipping"
        );
    }
}

/// One connect, register, drain, listen pass. `Ok` (reconnect now) on a graceful end or a
/// gone session; `Err` (back off) otherwise, including `Lagged`, which may have lost a wake.
async fn connect_and_serve_once(project: &str, session: &str, audit_path: &Path) -> Result<()> {
    // `client` must live for this whole function: dropping it closes the connection.
    let (client, project_id, pending) = tokio::time::timeout(NETWORK_CALL_DEADLINE, async {
        let client = nemonik_core::mcp::connect(
            ClientLifecycleMode::Discover { preferred_versions: vec![ProtocolVersion::V_2026_07_28] },
            (),
        )
        .await?;
        let peer = client.peer().clone();

        let project_id = harness_register(&peer, project, session).await?;
        let pending = harness_fetch_pending(&peer, project, session).await?;

        Ok::<_, anyhow::Error>((client, project_id, pending))
    })
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "timed out after {NETWORK_CALL_DEADLINE:?} connecting, registering, and fetching pending work"
        )
    })??;

    let peer = client.peer().clone();

    // Per connection: a local attach can't outlive the connection its chunks go over.
    let mut watch = WatchState::default();
    for raw in &pending {
        dispatch_one_bounded(&peer, project, session, raw, audit_path, &mut watch).await;
    }

    let result =
        serve_registered_session(&peer, project, project_id, session, audit_path, &mut watch).await;
    watch.stop_stream();
    result
}

async fn serve_registered_session(
    peer: &Peer<RoleClient>,
    project: &str,
    project_id: i64,
    session: &str,
    audit_path: &Path,
    watch: &mut WatchState,
) -> Result<()> {
    // Subscribe by project_id; the tool calls below still pass the project string.
    let expected_uri = harness_wake_uri(project_id, session);
    let filter = SubscriptionFilter::builder()
        .resource_subscription(expected_uri.clone())
        .build();
    let mut subscription = peer.listen(filter).await?;

    let mut liveness_check = tokio::time::interval(LIVENESS_CHECK_INTERVAL);
    liveness_check.tick().await; // first tick fires immediately; consume it so the interval starts counting from now

    loop {
        tokio::select! {
            notification = subscription.next() => {
                match notification? {
                    Some(ServerNotification::ResourceUpdatedNotification(n)) => {
                        if n.params.uri != expected_uri {
                            continue; // defensive: filter requests only this one resource
                        }
                        let pending = tokio::time::timeout(
                            NETWORK_CALL_DEADLINE,
                            harness_fetch_pending(peer, project, session),
                        )
                        .await
                        .map_err(|_| {
                            anyhow::anyhow!(
                                "timed out after {NETWORK_CALL_DEADLINE:?} fetching pending work"
                            )
                        })??;
                        for raw in &pending {
                            dispatch_one_bounded(peer, project, session, raw, audit_path, watch).await;
                        }
                    }
                    Some(_other) => continue,
                    None => {
                        return match subscription.end() {
                            Some(SubscriptionEnd::Graceful(_)) | Some(SubscriptionEnd::Cancelled) => Ok(()),
                            other => Err(anyhow::anyhow!("subscription ended abnormally: {other:?}")),
                        };
                    }
                }
            }
            _ = liveness_check.tick() => {
                if !tmux_has_session(session).await {
                    // End cleanly; the outer `run` loop decides whether to exit.
                    return Ok(());
                }
            }
        }
    }
}

/// Reimplements rmcp 3.1.2's `pub(crate)` `is_authorization_required`: walks `source()`
/// for either auth error type.
fn dynamic_transport_error_is_auth_required(error: &DynamicTransportError) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error.error.as_ref());
    while let Some(cause) = current {
        if matches!(
            cause.downcast_ref::<rmcp::transport::auth::AuthError>(),
            Some(rmcp::transport::auth::AuthError::AuthorizationRequired)
        ) {
            return true;
        }
        if cause.is::<AuthRequiredError>() {
            return true;
        }
        current = cause.source();
    }
    false
}

/// The server rejected the token (our local expiry clock can't tell). `call_tool` failures
/// hide the transport error inside `ServiceError::TransportSend` (no `#[source]`), so it
/// is unwrapped by hand; connect failures expose `is_authorization_required` directly.
fn is_auth_rejected(e: &anyhow::Error) -> bool {
    if let Some(ServiceError::TransportSend(transport_err)) = e.downcast_ref::<ServiceError>() {
        return dynamic_transport_error_is_auth_required(transport_err);
    }
    if let Some(init_err) = e.downcast_ref::<ClientInitializeError>() {
        return init_err.is_authorization_required();
    }
    false
}

/// The daemon loop: exits once the target session is gone, else reconnects with backoff.
pub async fn run(project: &str, session: &str) -> Result<()> {
    let audit_path = audit_log_path()?;
    let mut attempt: u32 = 0;

    loop {
        if !tmux_has_session(session).await {
            // The session-closed hook does this too; both are idempotent, and
            // the hook is best-effort.
            nemonik_core::launchspec::remove_launch_spec(session);
            if let Err(e) = nemonik_core::skills::cleanup(session) {
                eprintln!(
                    "harness daemon: failed to clean up materialized skills for '{session}': {e:#}"
                );
            }
            return Ok(());
        }

        match connect_and_serve_once(project, session, &audit_path).await {
            Ok(()) => attempt = 0,
            // A dead credential won't self-heal; keep retrying in case the user logs in again.
            Err(e)
                if e.downcast_ref::<nemonik_core::auth::NotLoggedIn>()
                    .is_some() =>
            {
                eprintln!("harness daemon: {e:#}");
                tokio::time::sleep(backoff_delay(attempt, jitter_seed_from_clock())).await;
                attempt = attempt.saturating_add(1);
            }
            // Reconnecting alone would reuse the rejected token, so force a refresh first.
            Err(e) if is_auth_rejected(&e) => {
                eprintln!(
                    "harness daemon: server rejected our access token, forcing a refresh: {e:#}"
                );
                if let Err(refresh_err) = nemonik_core::auth::force_refresh_access_token().await {
                    eprintln!("harness daemon: {refresh_err:#}");
                }
                tokio::time::sleep(backoff_delay(attempt, jitter_seed_from_clock())).await;
                attempt = attempt.saturating_add(1);
            }
            Err(e) => {
                eprintln!("harness daemon: {e:#}; retrying");
                tokio::time::sleep(backoff_delay(attempt, jitter_seed_from_clock())).await;
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::transport::streamable_http_client::StreamableHttpError;

    // The real `call_tool` failure shape: `ServiceError::TransportSend` wrapping the error.
    fn service_error_wrapping(inner: StreamableHttpError<reqwest::Error>) -> ServiceError {
        ServiceError::TransportSend(DynamicTransportError::from_parts(
            "streamable-http",
            std::any::TypeId::of::<()>(),
            Box::new(inner),
        ))
    }

    #[test]
    fn is_auth_rejected_detects_the_real_service_error_transport_send_shape() {
        let inner = StreamableHttpError::AuthRequired(AuthRequiredError::new("Bearer".to_string()));
        let err = anyhow::Error::new(service_error_wrapping(inner))
            .context("harness_fetch_pending failed");
        assert!(is_auth_rejected(&err));
    }

    #[test]
    fn is_auth_rejected_ignores_a_transport_send_error_that_is_not_auth_related() {
        let inner: StreamableHttpError<reqwest::Error> = StreamableHttpError::UnexpectedEndOfStream;
        let err = anyhow::Error::new(service_error_wrapping(inner))
            .context("harness_fetch_pending failed");
        assert!(!is_auth_rejected(&err));
    }

    #[test]
    fn is_auth_rejected_ignores_a_service_error_that_is_not_transport_send() {
        let err = anyhow::Error::new(ServiceError::TransportClosed)
            .context("harness_fetch_pending failed");
        assert!(!is_auth_rejected(&err));
    }

    #[test]
    fn is_auth_rejected_ignores_unrelated_failures() {
        let err = anyhow::anyhow!("subscription ended abnormally: Some(Abrupt)");
        assert!(!is_auth_rejected(&err));
    }

    #[test]
    fn extract_requests_reads_the_requests_array() {
        let raw = serde_json::json!({"requests": [{"request_id": "a"}]});
        assert_eq!(extract_requests(&raw).unwrap().len(), 1);
    }

    #[test]
    fn extract_requests_errors_when_requests_key_is_missing() {
        let raw = serde_json::json!({});
        assert!(extract_requests(&raw).is_err());
    }

    #[test]
    fn harness_wake_uri_format_matches_the_server_contract() {
        assert_eq!(
            harness_wake_uri(42, "my-session"),
            "harness://42/my-session"
        );
    }

    #[test]
    fn extract_project_id_reads_the_resolved_id() {
        let raw = serde_json::json!({"registered": true, "project_id": 42});
        assert_eq!(extract_project_id(&raw).unwrap(), 42);
    }

    #[test]
    fn extract_project_id_errors_when_missing_or_non_integer() {
        assert!(extract_project_id(&serde_json::json!({"registered": true})).is_err());
        assert!(extract_project_id(&serde_json::json!({"project_id": "42"})).is_err());
    }

    #[test]
    fn parse_pending_item_parses_a_real_execute_item() {
        // Exact shape confirmed live against nemonik's server.
        let raw = serde_json::json!({
            "request_id": "51258430-00f8-410c-bacb-885472a9a81e",
            "kind": "execute",
            "command": "echo hello from remote_write spike"
        });
        assert_eq!(
            parse_pending_item(&raw).unwrap(),
            PendingItem::Execute {
                request_id: "51258430-00f8-410c-bacb-885472a9a81e".to_string(),
                command: "echo hello from remote_write spike".to_string(),
            }
        );
    }

    #[test]
    fn parse_pending_item_parses_a_real_capture_item_with_null_command() {
        let raw = serde_json::json!({
            "request_id": "62c90011-a007-4888-9dcf-b3e1747e303b",
            "kind": "capture",
            "command": null
        });
        assert_eq!(
            parse_pending_item(&raw).unwrap(),
            PendingItem::Capture {
                request_id: "62c90011-a007-4888-9dcf-b3e1747e303b".to_string(),
                scrollback_lines: None,
            }
        );
    }

    #[test]
    fn parse_pending_item_treats_an_absent_capture_command_like_null() {
        let raw = serde_json::json!({"request_id": "a", "kind": "capture"});
        assert_eq!(
            parse_pending_item(&raw).unwrap(),
            PendingItem::Capture {
                request_id: "a".to_string(),
                scrollback_lines: None,
            }
        );
    }

    #[test]
    fn parse_pending_item_parses_a_capture_item_with_a_scrollback_lines_command() {
        let raw = serde_json::json!({
            "request_id": "a3333333-0000-0000-0000-000000000003",
            "kind": "capture",
            "command": "500"
        });
        assert_eq!(
            parse_pending_item(&raw).unwrap(),
            PendingItem::Capture {
                request_id: "a3333333-0000-0000-0000-000000000003".to_string(),
                scrollback_lines: Some(500),
            }
        );
    }

    #[test]
    fn parse_pending_item_errors_on_a_negative_scrollback_lines_command() {
        let raw = serde_json::json!({
            "request_id": "a",
            "kind": "capture",
            "command": "-5"
        });
        assert!(parse_pending_item(&raw).is_err());
    }

    #[test]
    fn parse_pending_item_errors_on_missing_request_id() {
        let raw = serde_json::json!({"kind": "execute", "command": "x"});
        assert!(parse_pending_item(&raw).is_err());
    }

    #[test]
    fn parse_pending_item_errors_on_unknown_kind() {
        let raw = serde_json::json!({"request_id": "a", "kind": "teleport"});
        assert!(parse_pending_item(&raw).is_err());
    }

    #[test]
    fn parse_pending_item_errors_when_execute_is_missing_command() {
        let raw = serde_json::json!({"request_id": "req-42", "kind": "execute", "command": null});
        let err = format!("{:#}", parse_pending_item(&raw).unwrap_err());
        // The log line is the only trace of a dropped item, so it must name the request.
        assert!(err.contains("req-42"), "{err}");
    }

    #[test]
    fn backoff_delay_doubles_then_caps_at_30s() {
        assert_eq!(backoff_delay(1, 0), Duration::from_millis(1000));
        assert_eq!(backoff_delay(2, 0), Duration::from_millis(2000));
        assert_eq!(backoff_delay(20, 0), Duration::from_millis(30_000));
    }

    #[test]
    fn backoff_delay_jitter_stays_within_25_percent_and_never_hits_zero() {
        let base = backoff_delay(3, 0); // 4000ms, no jitter
        let jittered = backoff_delay(3, u32::MAX); // max jitter
        assert!(jittered <= base);
        assert!(jittered >= base - base / 4);
        assert!(jittered > Duration::ZERO);
    }

    #[test]
    fn audit_log_line_truncates_long_commands_on_a_char_boundary() {
        let long_command: String = "é".repeat(AUDIT_LOG_MAX_COMMAND_CHARS + 10);
        let line = audit_log_line("req-1", &long_command, SystemTime::UNIX_EPOCH);
        assert!(line.is_char_boundary(line.len()));
        assert!(line.len() < long_command.len() * 2);
    }

    #[test]
    fn result_shapes_match_the_server_contract() {
        assert_eq!(result_for_execute(), serde_json::json!({"delivered": true}));
        assert_eq!(
            result_for_capture("pane text"),
            serde_json::json!({"pane": "pane text"})
        );
    }

    #[test]
    fn audit_log_line_never_produces_more_than_one_physical_line() {
        let line = audit_log_line(
            "req-1",
            "line one\nline two\r\nline three",
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(line.lines().count(), 1);
    }

    #[test]
    fn pump_step_buffers_output_and_produces_no_action_until_a_tick_flushes_it() {
        let (buffer, actions) = pump_step(
            String::new(),
            PumpInput::Event(control_mode::StreamEvent::Output {
                pane_id: "%1".to_string(),
                text: "hi".to_string(),
            }),
        );
        assert_eq!(buffer, "hi");
        assert!(actions.is_empty());

        let (buffer, actions) = pump_step(buffer, PumpInput::Tick);
        assert_eq!(buffer, "");
        assert_eq!(actions, vec![WatchChunk::Data("hi".to_string())]);
    }

    #[test]
    fn pump_step_flushes_early_once_the_buffer_hits_the_max_size() {
        let text = "x".repeat(WATCH_MAX_BUFFER_BYTES);
        let (buffer, actions) = pump_step(
            String::new(),
            PumpInput::Event(control_mode::StreamEvent::Output {
                pane_id: "%1".to_string(),
                text: text.clone(),
            }),
        );
        assert_eq!(buffer, "");
        assert_eq!(actions, vec![WatchChunk::Data(text)]);
    }

    #[test]
    fn pump_step_flushes_a_pending_buffer_before_an_initial_snapshot_to_preserve_order() {
        let (buffer, _) = pump_step(
            String::new(),
            PumpInput::Event(control_mode::StreamEvent::Output {
                pane_id: "%1".to_string(),
                text: "earlier".to_string(),
            }),
        );
        let (buffer, actions) = pump_step(
            buffer,
            PumpInput::Event(control_mode::StreamEvent::InitialSnapshot(vec![
                "line one".to_string(),
                "line two".to_string(),
            ])),
        );
        assert_eq!(buffer, "");
        assert_eq!(
            actions,
            vec![
                WatchChunk::Data("earlier".to_string()),
                WatchChunk::Data("line one\nline two".to_string())
            ]
        );
    }

    #[test]
    fn pump_step_flushes_and_ends_on_stream_ended() {
        let (buffer, _) = pump_step(
            String::new(),
            PumpInput::Event(control_mode::StreamEvent::Output {
                pane_id: "%1".to_string(),
                text: "tail".to_string(),
            }),
        );
        let (buffer, actions) =
            pump_step(buffer, PumpInput::Event(control_mode::StreamEvent::Ended));
        assert_eq!(buffer, "");
        assert_eq!(
            actions,
            vec![WatchChunk::Data("tail".to_string()), WatchChunk::Ended]
        );
    }

    #[test]
    fn note_watch_stop_decrements_refcount_and_only_stops_the_stream_at_zero() {
        let mut watch = WatchState {
            handle: None,
            chunk_task: None,
            refcount: 2,
        };
        note_watch_stop(&mut watch);
        assert_eq!(watch.refcount, 1);
        note_watch_stop(&mut watch);
        assert_eq!(watch.refcount, 0);
    }

    #[test]
    fn note_watch_stop_saturates_rather_than_underflowing_on_an_unmatched_stop() {
        let mut watch = WatchState::default();
        note_watch_stop(&mut watch);
        assert_eq!(watch.refcount, 0);
    }
}
