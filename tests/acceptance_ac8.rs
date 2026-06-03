//! AC8 (MUST): AT-SPI bus health — install.sh enables the accessibility bus
//! for a fresh user session so `wm-desktop apps` succeeds on the next launch.
//!
//! These tests verify the install.sh artefact exists and contains the required
//! AT-SPI enablement logic. They are pure file-content assertions; no X11 or
//! D-Bus socket is required.

#[test]
fn install_sh_exists_and_is_executable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh");
    assert!(
        path.exists(),
        "install.sh must exist at repo root (PRD AC8)"
    );

    // Check executable bit is set (Unix permission).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&path).expect("stat install.sh");
        let mode = meta.permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "install.sh must have executable bit set (mode={:o})",
            mode
        );
    }
}

#[test]
fn install_sh_enables_atspi_systemd_service() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh");
    let content = std::fs::read_to_string(&path).expect("read install.sh");

    assert!(
        content.contains("at-spi-dbus-bus.service"),
        "install.sh must reference at-spi-dbus-bus.service for systemd user enable"
    );
    assert!(
        content.contains("systemctl --user enable"),
        "install.sh must run 'systemctl --user enable' to activate AT-SPI on login"
    );
}

#[test]
fn install_sh_writes_xdg_autostart_desktop_file() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh");
    let content = std::fs::read_to_string(&path).expect("read install.sh");

    assert!(
        content.contains("autostart"),
        "install.sh must write an XDG autostart entry as fallback for non-systemd DEs"
    );
    assert!(
        content.contains("at-spi-bus-launcher"),
        "install.sh must reference at-spi-bus-launcher in the XDG autostart entry"
    );
    assert!(
        content.contains("[Desktop Entry]"),
        "install.sh must write a valid .desktop file containing [Desktop Entry]"
    );
}

#[test]
fn install_sh_builds_and_installs_wm_desktop_binary() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh");
    let content = std::fs::read_to_string(&path).expect("read install.sh");

    assert!(
        content.contains("cargo install --path ."),
        "install.sh must build wm-desktop via cargo install"
    );
    assert!(
        content.contains("wm-desktop"),
        "install.sh must reference the wm-desktop binary"
    );
}
