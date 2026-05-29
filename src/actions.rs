//! Action execution via the `baton` subprocess wrapper.
//!
//! `baton` (at `~/.local/bin/baton`) handles X11 window-id resolution and
//! keystroke envelope delivery via xdotool. This module is a thin async
//! wrapper that shells out to `baton` commands.
//!
//! All functions accept a `dry_run` flag (used in tests) that prints the
//! command that would be executed without actually running it.

use anyhow::{Context as _, Result, bail};
use tokio::process::Command;

/// Execute `baton type <text>` in the focused window.
///
/// When `dry_run` is true, the command is constructed but not executed;
/// the function returns `Ok(())` immediately. This allows acceptance
/// tests to verify argument construction without requiring `baton` to
/// be installed.
///
/// # Errors
///
/// Returns `Err` if the `baton` subprocess fails to launch or exits
/// with a non-zero status.
pub async fn baton_type(text: &str, dry_run: bool) -> Result<()> {
    let mut cmd = Command::new("baton");
    cmd.arg("type").arg(text);
    run_baton(cmd, "baton type", dry_run).await
}

/// Execute `baton key <combo>` in the focused window.
///
/// # Errors
///
/// Returns `Err` if the `baton` subprocess fails to launch or exits
/// with a non-zero status.
pub async fn baton_key(combo: &str, dry_run: bool) -> Result<()> {
    let mut cmd = Command::new("baton");
    cmd.arg("key").arg(combo);
    run_baton(cmd, "baton key", dry_run).await
}

/// Execute `baton focus <window_spec>` to bring a window to the foreground.
///
/// `window_spec` may be a window ID (decimal or `0x`-prefixed hex) or an
/// application name; baton resolves the target via xdotool search.
///
/// # Errors
///
/// Returns `Err` if the `baton` subprocess fails to launch or exits
/// with a non-zero status.
pub async fn baton_focus(window_spec: &str, dry_run: bool) -> Result<()> {
    let mut cmd = Command::new("baton");
    cmd.arg("focus").arg(window_spec);
    run_baton(cmd, "baton focus", dry_run).await
}

async fn run_baton(mut cmd: Command, label: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }
    let status = cmd
        .status()
        .await
        .with_context(|| format!("{label}: failed to launch baton"))?;
    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(-1);
        bail!("{label}: exited with status {code}");
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "tests: assertions are the point"
)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn baton_type_dry_run_succeeds() {
        let result = baton_type("hello world", true).await;
        assert!(result.is_ok(), "dry_run should always succeed: {result:?}");
    }

    #[tokio::test]
    async fn baton_key_dry_run_succeeds() {
        let result = baton_key("ctrl+s", true).await;
        assert!(result.is_ok(), "dry_run should always succeed: {result:?}");
    }

    #[tokio::test]
    async fn baton_focus_dry_run_succeeds() {
        let result = baton_focus("firefox", true).await;
        assert!(result.is_ok(), "dry_run should always succeed: {result:?}");
    }
}
