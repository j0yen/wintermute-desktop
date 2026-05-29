//! AC5 (MUST): Daemon dispatches `apps`/`focus`/`read_window`/`click`/`type`/`key`/`find`
//! commands; publishes replies on the correct topic. Unknown tool → `{ok:false,error:"unknown tool"}`
//! rather than panic.
//!
//! Tests the `dispatch` function directly — no agorabus socket or AT-SPI bus required.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "acceptance tests: assertions are the point"
)]

use serde_json::json;
use wintermute_desktop::daemon;
use wintermute_desktop::protocol::Command;

fn cmd(tool: &str, args: serde_json::Value) -> Command {
    Command {
        cmd_id: format!("test-{tool}"),
        tool: tool.to_string(),
        args,
    }
}

#[tokio::test]
async fn dispatch_unknown_tool_returns_structured_error() {
    let c = cmd("totally_unknown_tool_xyz", json!({}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(!reply.ok, "unknown tool must return ok=false");
    let err = reply.error.as_deref().unwrap_or("");
    assert!(
        err.contains("unknown tool"),
        "error message should say 'unknown tool', got: '{err}'"
    );
}

#[tokio::test]
async fn dispatch_apps_without_atspi_returns_structured_error() {
    let c = cmd("apps", json!({}));
    // Pass None for AT-SPI connection — daemon must not panic
    let reply = daemon::dispatch(&c, None).await;
    // AT-SPI unavailable → ok=false with a descriptive error, not a panic
    assert!(!reply.ok, "apps with no AT-SPI must return ok=false");
    assert!(
        reply.error.is_some(),
        "error field must be present when AT-SPI unavailable"
    );
}

#[tokio::test]
async fn dispatch_read_window_without_atspi_returns_structured_error() {
    let c = cmd("read_window", json!({}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(!reply.ok);
    assert!(reply.error.is_some());
}

#[tokio::test]
async fn dispatch_focus_missing_args_returns_structured_error() {
    let c = cmd("focus", json!({}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(!reply.ok);
    let err = reply.error.as_deref().unwrap_or("");
    assert!(
        err.contains("focus"),
        "error message should mention 'focus', got: '{err}'"
    );
}

#[tokio::test]
async fn dispatch_type_missing_text_returns_structured_error() {
    let c = cmd("type", json!({}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(!reply.ok);
    let err = reply.error.as_deref().unwrap_or("");
    assert!(
        err.contains("text"),
        "error should mention 'text', got: '{err}'"
    );
}

#[tokio::test]
async fn dispatch_key_missing_combo_returns_structured_error() {
    let c = cmd("key", json!({}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(!reply.ok);
    let err = reply.error.as_deref().unwrap_or("");
    assert!(
        err.contains("combo"),
        "error should mention 'combo', got: '{err}'"
    );
}

#[tokio::test]
async fn dispatch_find_missing_query_returns_structured_error() {
    let c = cmd("find", json!({}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(!reply.ok);
    let err = reply.error.as_deref().unwrap_or("");
    assert!(
        err.contains("query"),
        "error should mention 'query', got: '{err}'"
    );
}

#[tokio::test]
async fn dispatch_find_with_valid_query_returns_ok() {
    let c = cmd("find", json!({"query": "cancel", "role": "button"}));
    let reply = daemon::dispatch(&c, None).await;
    // find operates on empty snapshot store in iter-1, returns ok with empty matches
    assert!(reply.ok, "find with valid args must return ok=true");
    let matches = reply.result.as_ref().and_then(|r| r["matches"].as_array());
    assert!(matches.is_some(), "result must contain 'matches' array");
}

#[tokio::test]
async fn dispatch_type_dry_run_returns_ok() {
    let c = cmd("type", json!({"text": "hello world", "dry_run": true}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(reply.ok, "type dry_run must return ok=true, got: {reply:?}");
}

#[tokio::test]
async fn dispatch_key_dry_run_returns_ok() {
    let c = cmd("key", json!({"combo": "ctrl+s", "dry_run": true}));
    let reply = daemon::dispatch(&c, None).await;
    assert!(reply.ok, "key dry_run must return ok=true, got: {reply:?}");
}

#[tokio::test]
async fn dispatch_echoes_cmd_id_in_reply() {
    // All replies must echo the originating cmd_id
    let c = cmd("find", json!({"query": "test"}));
    let reply = daemon::dispatch(&c, None).await;
    assert_eq!(
        reply.cmd_id, c.cmd_id,
        "reply.cmd_id must echo command.cmd_id"
    );
}

#[tokio::test]
async fn dispatch_click_without_ref_returns_structured_error() {
    let c = cmd("click", json!({}));
    let reply = daemon::dispatch(&c, None).await;
    // click is not yet fully implemented; must not panic and must return structured error
    assert!(!reply.ok, "click without ref must return ok=false");
    assert!(
        reply.error.is_some(),
        "click must include error message, got: {reply:?}"
    );
}
