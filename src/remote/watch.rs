//! `nemonik watch`: streams a session's live output from `remote_watch`. Chunks
//! arrive as progress notifications, so this uses `send_cancellable_request`, not
//! `call_tool`. Output is raw pane bytes, so the terminal is reset on every exit.

use anyhow::{Context, Result};
use futures::StreamExt;
use rmcp::ClientLifecycleMode;
use rmcp::model::{ClientRequest, ProtocolVersion, Request, ServerResult};
use rmcp::service::PeerRequestOptions;
use std::io::Write as _;

pub async fn watch(project: &str, session: &str) -> Result<()> {
    let client = nemonik_core::mcp::connect(
        ClientLifecycleMode::Discover {
            preferred_versions: vec![ProtocolVersion::V_2026_07_28],
        },
        nemonik_core::mcp::ProgressAwareHandler::default(),
    )
    .await?;

    let mut args = serde_json::Map::new();
    args.insert("project".into(), project.into());
    args.insert("session".into(), session.into());
    let params = nemonik_core::mcp::tool_params("remote_watch", args);
    let request = ClientRequest::CallToolRequest(Request::new(params));

    let mut handle = client
        .send_cancellable_request(request, PeerRequestOptions::no_options())
        .await
        .context("remote_watch request failed to send")?;

    // Subscribing after the send can drop a very early chunk (rmcp doesn't buffer), but
    // the first chunk's round trip makes that window negligible; rmcp's tests do the same.
    let mut chunks = client
        .service()
        .progress
        .subscribe(handle.progress_token.clone())
        .await;

    // Raw bytes, unframed: the watcher renders the pane's own output.
    let forward = tokio::spawn(async move {
        let mut stdout = std::io::stdout();
        while let Some(notification) = chunks.next().await {
            if let Some(text) = notification.message {
                let _ = stdout.write_all(text.as_bytes());
                let _ = stdout.flush();
            }
        }
    });

    // Race completion against Ctrl-C so the call can be cancelled server-side. Polls
    // `handle.rx` (borrowing `handle`) so `handle.cancel` is still available after.
    let outcome = tokio::select! {
        result = &mut handle.rx => Some(result),
        _ = tokio::signal::ctrl_c() => None,
    };

    forward.abort();
    reset_terminal();

    let response = match outcome {
        Some(result) => result
            .map_err(|_e| anyhow::anyhow!("remote_watch call failed: transport closed"))?
            .context("remote_watch call failed")?,
        None => {
            // Cancel explicitly: `remote_watch` has no timeout, so the server would keep
            // the watcher attached forever.
            let _ = handle
                .cancel(Some("user interrupted (Ctrl-C)".to_string()))
                .await;
            println!("(stopped watching)");
            return Ok(());
        }
    };

    match response {
        ServerResult::CallToolResult(_) => {
            println!("(watch ended)");
            Ok(())
        }
        other => anyhow::bail!("remote_watch returned an unexpected response: {other:?}"),
    }
}

/// Undoes mode changes the watched pane's raw bytes leaked: alternate screen, bracketed
/// paste, hidden cursor, SGR.
fn reset_terminal() {
    print!("\x1b[?1049l\x1b[?2004l\x1b[?25h\x1b[0m");
    let _ = std::io::stdout().flush();
}
