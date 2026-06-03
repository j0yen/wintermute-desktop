#!/usr/bin/env bash
# install.sh — install wm-desktop (wintermute AT-SPI desktop daemon).
#
# Covers PRD AC8: on a fresh user session with accessibility off, this script
# enables the AT-SPI bus via systemd user service + XDG autostart, so that
# `wm-desktop apps` succeeds on the next launch.
#
# Modes:
#   1. Repo-local: invoked as `./install.sh` from a checkout.
#   2. Curl-piped: invoked as `curl ... | bash`. Self-clones into
#      ~/.local/share/wintermute-desktop/ then continues.

set -euo pipefail

SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
SCRIPT_DIR=""
if [ -f "$SCRIPT_PATH" ]; then
  SCRIPT_DIR=$(cd "$(dirname "$SCRIPT_PATH")" && pwd)
fi

if [ -z "$SCRIPT_DIR" ] || [ ! -f "$SCRIPT_DIR/Cargo.toml" ] \
   || ! grep -q '^name = "wm-desktop"' "$SCRIPT_DIR/Cargo.toml" 2>/dev/null; then
  echo "-> self-cloning j0yen/wintermute-desktop..."
  command -v git >/dev/null 2>&1 || { echo "fatal: git not found"; exit 1; }

  CLONE_ROOT="${WM_DESKTOP_CLONE_ROOT:-$HOME/.local/share/wintermute-desktop}"
  mkdir -p "$(dirname "$CLONE_ROOT")"

  if [ -d "$CLONE_ROOT/.git" ]; then
    echo "-> existing clone at $CLONE_ROOT -- refreshing"
    git -C "$CLONE_ROOT" fetch --depth 1 origin main
    git -C "$CLONE_ROOT" reset --hard origin/main
  else
    git clone --depth 1 https://github.com/j0yen/wintermute-desktop.git "$CLONE_ROOT"
  fi

  SCRIPT_DIR="$CLONE_ROOT"
fi

cd "$SCRIPT_DIR"

command -v cargo >/dev/null 2>&1 || {
  echo "fatal: cargo not found. Install Rust: https://rustup.rs/"
  exit 1
}

echo "-> building + installing wm-desktop via cargo install (this can take a few minutes)..."
cargo install --path . --locked

if ! command -v wm-desktop >/dev/null 2>&1; then
  echo
  echo "! wm-desktop installed but not on PATH. Add ~/.cargo/bin to PATH:"
  echo "    export PATH=\"\$HOME/.cargo/bin:\$PATH\""
fi

# ---------------------------------------------------------------------------
# AC8: AT-SPI bus health — enable accessibility bus for the user session.
# ---------------------------------------------------------------------------
# Two complementary mechanisms:
#   1. systemd user service: at-spi-dbus-bus.service (ships with at-spi2-core).
#      Enable it so the bus starts on login.
#   2. XDG autostart: write ~/.config/autostart/at-spi-dbus-bus.desktop if the
#      system-level file is missing (covers DEs that don't use systemd --user).
# ---------------------------------------------------------------------------

echo "-> enabling AT-SPI accessibility bus..."

ATSPI_SERVICE="at-spi-dbus-bus.service"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
AUTOSTART_FILE="$AUTOSTART_DIR/at-spi-dbus-bus.desktop"

# 1. systemd user service (best path on modern desktops).
if systemctl --user list-unit-files "$ATSPI_SERVICE" 2>/dev/null | grep -q "$ATSPI_SERVICE"; then
  if ! systemctl --user is-enabled "$ATSPI_SERVICE" >/dev/null 2>&1; then
    systemctl --user enable "$ATSPI_SERVICE" && \
      echo "   enabled $ATSPI_SERVICE via systemd --user"
  else
    echo "   $ATSPI_SERVICE already enabled"
  fi
else
  echo "   $ATSPI_SERVICE not found via systemctl; skipping systemd enable"
fi

# 2. XDG autostart (fallback for XFCE, LXDE, etc.).
ATSPI_LAUNCHER=""
for candidate in \
  "/usr/lib/at-spi2-core/at-spi-bus-launcher" \
  "/usr/libexec/at-spi-bus-launcher" \
  "/usr/lib/at-spi-bus-launcher"; do
  if [ -x "$candidate" ]; then
    ATSPI_LAUNCHER="$candidate"
    break
  fi
done

if [ -n "$ATSPI_LAUNCHER" ] && [ ! -f "$AUTOSTART_FILE" ]; then
  mkdir -p "$AUTOSTART_DIR"
  cat > "$AUTOSTART_FILE" <<DESKTOP_EOF
[Desktop Entry]
Type=Application
Name=AT-SPI D-Bus Bus
Comment=Accessibility bus launcher for AT-SPI
Exec=$ATSPI_LAUNCHER --launch-immediately
NoDisplay=true
OnlyShowIn=
X-GNOME-Autostart-enabled=true
DESKTOP_EOF
  echo "   wrote XDG autostart: $AUTOSTART_FILE"
elif [ -f "$AUTOSTART_FILE" ]; then
  echo "   XDG autostart already present: $AUTOSTART_FILE"
else
  echo "   at-spi-bus-launcher not found in standard paths; skipping XDG autostart"
fi

# 3. Set org.gnome.desktop.interface toolkit-accessibility=true if gsettings is
#    available (enables AT-SPI for GTK3/4 apps without env var).
if command -v gsettings >/dev/null 2>&1; then
  if ! gsettings get org.gnome.desktop.interface toolkit-accessibility 2>/dev/null | grep -q "true"; then
    gsettings set org.gnome.desktop.interface toolkit-accessibility true 2>/dev/null && \
      echo "   set org.gnome.desktop.interface toolkit-accessibility=true" || \
      echo "   gsettings set failed (non-GNOME DE?); continuing"
  else
    echo "   toolkit-accessibility already true"
  fi
fi

echo "-> AT-SPI setup done. Re-login or run 'systemctl --user start $ATSPI_SERVICE' to activate."
echo "-> wm-desktop install complete."
