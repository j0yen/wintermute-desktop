//! `wm-desktop` — CLI entry point.
//!
//! Two surfaces over the same dispatcher:
//!   * `wm-desktop daemon` — run the long-lived agorabus subscriber.
//!   * `wm-desktop <tool> [args]` — one-shot mode: print the JSON reply and
//!     exit. Routes through the same [`daemon::dispatch`] dispatcher the
//!     daemon uses so argument handling and result shape match exactly.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::json;
use tracing_subscriber::EnvFilter;
use wintermute_desktop::daemon;
use wintermute_desktop::protocol::{Command, Tool};

#[derive(Parser)]
#[command(name = "wm-desktop", version, about = "AT-SPI desktop access for the wintermute brain")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the long-lived daemon: subscribe to `wm.desktop.cmd` and dispatch.
    Daemon,
    /// List running accessible applications.
    Apps,
    /// Focus a window or application by name or window-id.
    Focus {
        /// Application name or window-id to focus.
        target: String,
    },
    /// Read the AT-SPI tree of the currently focused window.
    ReadWindow {
        /// Optional window-id filter (currently unused; reads focused window).
        #[arg(long)]
        window_id: Option<String>,
    },
    /// Filter the latest snapshot by text + optional role.
    Find {
        /// Case-insensitive query matched against role and name.
        query: String,
        /// Optional role filter (button, textfield, label, …).
        #[arg(long)]
        role: Option<String>,
    },
    /// Type text into the focused window via baton.
    Type {
        /// Text to type.
        text: String,
        /// Dry-run: construct the command but don't execute baton.
        #[arg(long)]
        dry_run: bool,
    },
    /// Send a key combination via baton (e.g. `ctrl+s`).
    Key {
        /// Key combo string (e.g. `ctrl+s`).
        combo: String,
        /// Dry-run: construct the command but don't execute baton.
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
#[allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "main: user-facing output and error reporting"
)]
async fn main() -> ExitCode {
    // Initialize tracing; default to INFO unless RUST_LOG overrides.
    let directive = "wm_desktop=info"
        .parse()
        .unwrap_or_else(|_| tracing_subscriber::filter::LevelFilter::INFO.into());
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(directive))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Cmd::Daemon => match daemon::run().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("wm-desktop daemon error: {e:#}");
                ExitCode::FAILURE
            }
        },
        one_shot => match run_one_shot(one_shot).await {
            Ok(out) => {
                println!("{out}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("wm-desktop error: {e:#}");
                ExitCode::FAILURE
            }
        },
    }
}

/// Run a one-shot tool call: build a [`Command`] envelope, dispatch through
/// the daemon logic, and return the serialized JSON reply.
async fn run_one_shot(cmd: Cmd) -> anyhow::Result<String> {
    let (tool_str, args) = cmd_to_args(cmd)?;
    let cmd_id = uuid::Uuid::new_v4().to_string();

    let envelope = Command {
        cmd_id: cmd_id.clone(),
        tool: tool_str,
        args,
    };

    // Establish AT-SPI connection for tools that need it.
    let atspi = atspi_connection::AccessibilityConnection::new().await.ok();

    let reply = daemon::dispatch(&envelope, atspi.as_ref()).await;
    let s = serde_json::to_string_pretty(&reply)?;
    Ok(s)
}

/// Convert a one-shot [`Cmd`] variant into a `(tool_name, args_json)` pair
/// suitable for a [`Command`] envelope.
///
/// # Errors
///
/// Returns `Err` if the `Cmd::Daemon` variant is passed (should never happen).
fn cmd_to_args(cmd: Cmd) -> anyhow::Result<(String, serde_json::Value)> {
    let pair = match cmd {
        Cmd::Daemon => anyhow::bail!("daemon is handled before one-shot path"),
        Cmd::Apps => (Tool::Apps.as_str().to_string(), json!({})),
        Cmd::Focus { target } => (
            Tool::Focus.as_str().to_string(),
            json!({ "app": target }),
        ),
        Cmd::ReadWindow { window_id } => {
            let args = window_id.map_or_else(|| json!({}), |id| json!({ "window_id": id }));
            (Tool::ReadWindow.as_str().to_string(), args)
        }
        Cmd::Find { query, role } => {
            let args = role.map_or_else(
                || json!({ "query": query }),
                |r| json!({ "query": query, "role": r }),
            );
            (Tool::Find.as_str().to_string(), args)
        }
        Cmd::Type { text, dry_run } => (
            Tool::Type.as_str().to_string(),
            json!({ "text": text, "dry_run": dry_run }),
        ),
        Cmd::Key { combo, dry_run } => (
            Tool::Key.as_str().to_string(),
            json!({ "combo": combo, "dry_run": dry_run }),
        ),
    };
    Ok(pair)
}
