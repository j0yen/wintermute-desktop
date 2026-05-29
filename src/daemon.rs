//! Agorabus subscribe loop and tool dispatcher.
//!
//! The daemon subscribes to `wm.desktop.cmd`, dispatches tool calls to the
//! AT-SPI / baton backends, and publishes replies on `wm.desktop.reply`.
//!
//! Mirrors `wm-browser`'s shape so the brain has one consistent mental model.

use anyhow::{Context as _, Result};
use serde_json::{Value, json};
use tracing::{error, info, warn};

use crate::actions::{baton_focus, baton_key, baton_type};
use crate::protocol::{Command, Reply, Tool, find_matches};
use crate::snapshot::{build_snapshot, list_apps};

/// Topics used on agorabus.
pub const CMD_TOPIC: &str = "wm.desktop.cmd";
/// Reply topic for all tool results.
pub const REPLY_TOPIC: &str = "wm.desktop.reply";

/// Run the daemon: connect to agorabus, subscribe to [`CMD_TOPIC`], and
/// dispatch commands until the process receives SIGINT/SIGTERM.
///
/// # Errors
///
/// Returns `Err` if the agorabus socket cannot be reached or if AT-SPI
/// initialization fails.
pub async fn run() -> Result<()> {
    let socket_path = agorabus::default_socket_path();
    info!("connecting to agorabus at {}", socket_path.display());

    let mut bus = agorabus::Client::connect(&socket_path)
        .await
        .context("connecting to agorabus")?;

    bus.announce(
        &format!("wm-desktop-{}", std::process::id()),
        std::process::id(),
        &std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string(),
        "wm-desktop daemon",
    )
    .await
    .context("announcing to agorabus")?;

    bus.subscribe(CMD_TOPIC)
        .await
        .context("subscribing to wm.desktop.cmd")?;

    info!("subscribed to {CMD_TOPIC}; waiting for commands");

    // Establish AT-SPI connection (fail-soft: apps/read_window fail gracefully
    // if AT-SPI is unavailable rather than crashing the daemon).
    let atspi = match atspi_connection::AccessibilityConnection::new().await {
        Ok(c) => {
            info!("AT-SPI connection established");
            Some(c)
        }
        Err(e) => {
            warn!(
                "AT-SPI unavailable ({}); apps/read_window tools will return error replies",
                e
            );
            None
        }
    };

    loop {
        let event = match bus.next_event().await {
            Ok(Some(ev)) => ev,
            Ok(None) => {
                info!("agorabus connection closed");
                break;
            }
            Err(e) => {
                error!("bus read error: {e}");
                break;
            }
        };

        // Parse the publish event into a Command envelope.
        let cmd: Command = match serde_json::from_value(event.data.clone()) {
            Ok(c) => c,
            Err(e) => {
                warn!("ignoring malformed command envelope: {e}");
                continue;
            }
        };

        let reply = dispatch(&cmd, atspi.as_ref()).await;

        // Publish the reply; log on failure but keep the loop alive.
        if let Err(e) = bus
            .publish(REPLY_TOPIC, serde_json::to_value(&reply).unwrap_or_default())
            .await
        {
            error!("failed to publish reply: {e}");
        }
    }

    Ok(())
}

/// Dispatch a single [`Command`] to the appropriate backend and return the
/// [`Reply`].  Pure `async` function — no side effects on the bus.
///
/// Unknown tools return `{ok: false, error: "unknown tool"}` rather than
/// panicking (intent-card AC5).
pub async fn dispatch(cmd: &Command, atspi: Option<&atspi_connection::AccessibilityConnection>) -> Reply {
    match Tool::parse(&cmd.tool) {
        None => Reply::err(&cmd.cmd_id, "unknown tool"),
        Some(Tool::Apps) => run_apps(cmd, atspi).await,
        Some(Tool::Focus) => run_focus(cmd).await,
        Some(Tool::ReadWindow) => run_read_window(cmd, atspi).await,
        Some(Tool::Click) => run_click(cmd),
        Some(Tool::Type) => run_type(cmd).await,
        Some(Tool::Key) => run_key(cmd).await,
        Some(Tool::Find) => run_find(cmd),
    }
}

// ---------------------------------------------------------------------------
// Tool handlers
// ---------------------------------------------------------------------------

async fn run_apps(
    cmd: &Command,
    atspi: Option<&atspi_connection::AccessibilityConnection>,
) -> Reply {
    let Some(conn) = atspi else {
        return Reply::err(
            &cmd.cmd_id,
            "AT-SPI unavailable; ensure accessibility bus is running",
        );
    };
    match list_apps(conn).await {
        Ok(apps) => Reply::ok(&cmd.cmd_id, json!({ "apps": apps })),
        Err(e) => Reply::err(&cmd.cmd_id, e.to_string()),
    }
}

async fn run_focus(cmd: &Command) -> Reply {
    let window_spec = match cmd.args.get("app").or_else(|| cmd.args.get("window_id")) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Reply::err(
                &cmd.cmd_id,
                "focus requires {app: string} or {window_id: string}",
            );
        }
    };

    match baton_focus(&window_spec, false).await {
        Ok(()) => Reply::ok(
            &cmd.cmd_id,
            json!({ "ok": true, "window_id": window_spec }),
        ),
        Err(e) => Reply::err(&cmd.cmd_id, e.to_string()),
    }
}

async fn run_read_window(
    cmd: &Command,
    atspi: Option<&atspi_connection::AccessibilityConnection>,
) -> Reply {
    let Some(conn) = atspi else {
        return Reply::err(
            &cmd.cmd_id,
            "AT-SPI unavailable; ensure accessibility bus is running",
        );
    };
    match build_snapshot(conn).await {
        Ok(snap) => match serde_json::to_value(&snap) {
            Ok(v) => Reply::ok(&cmd.cmd_id, v),
            Err(e) => Reply::err(&cmd.cmd_id, format!("snapshot serialization: {e}")),
        },
        Err(e) => Reply::err(&cmd.cmd_id, e.to_string()),
    }
}

fn run_click(cmd: &Command) -> Reply {
    // `click` resolves a snapshot ref → accessible action. Since we store
    // refs but not a live ref-map in this iteration, we return a structured
    // error telling the caller we need the ref from a fresh snapshot.
    let _ref_val = cmd.args.get("ref");
    Reply::err(
        &cmd.cmd_id,
        "click requires a live ref from read_window; dispatch not yet implemented",
    )
}

async fn run_type(cmd: &Command) -> Reply {
    let text = match cmd.args.get("text") {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Reply::err(&cmd.cmd_id, "type requires {text: string}");
        }
    };

    let dry_run = cmd
        .args
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match baton_type(&text, dry_run).await {
        Ok(()) => Reply::ok(&cmd.cmd_id, json!({ "ok": true })),
        Err(e) => Reply::err(&cmd.cmd_id, e.to_string()),
    }
}

async fn run_key(cmd: &Command) -> Reply {
    let combo = match cmd.args.get("combo") {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Reply::err(&cmd.cmd_id, "key requires {combo: string}");
        }
    };

    let dry_run = cmd
        .args
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match baton_key(&combo, dry_run).await {
        Ok(()) => Reply::ok(&cmd.cmd_id, json!({ "ok": true })),
        Err(e) => Reply::err(&cmd.cmd_id, e.to_string()),
    }
}

fn run_find(cmd: &Command) -> Reply {
    let query = match cmd.args.get("query") {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Reply::err(&cmd.cmd_id, "find requires {query: string}");
        }
    };

    let role_filter = cmd
        .args
        .get("role")
        .and_then(Value::as_str)
        .map(str::to_string);

    // `find` operates on the last snapshot if one exists. In daemon mode we
    // keep no persistent snapshot store in iter-1 — clients must call
    // `read_window` first and then `find` in a second command. For now we
    // return an empty match set so the shape is correct.
    let matches = find_matches(&[], &query, role_filter.as_deref());
    Reply::ok(&cmd.cmd_id, json!({ "matches": matches }))
}
