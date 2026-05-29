# wintermute-desktop

AT-SPI accessibility tree reader + xdotool/baton keystroke injector daemon
giving the wintermute brain a read+act surface on the X11 desktop.

## TL;DR

`wm-desktop` is a daemon that gives the brain a read+act surface on the
running X11 desktop: AT-SPI accessibility tree as the read mode, xdotool
(via the `baton` wrapper) as the act mode. Mirrors `wm-browser`'s shape
(tools over agorabus, snapshot-with-refs) so the brain has one consistent
mental model for browser and native-desktop surfaces.

## Tools (topic `wm.desktop.cmd`)

| Tool | Args | Returns |
|---|---|---|
| `apps` | `{}` | `{apps:[{name, pid, window_ids}]}` |
| `focus` | `{app or window_id}` | `{ok, window_id}` |
| `read_window` | `{window_id?}` | `{snapshot, snapshot_id}` — AT-SPI tree of focused window |
| `click` | `{ref}` | `{ok}` — resolves ref to action+target |
| `type` | `{text}` | `{ok}` — into focused window via baton |
| `key` | `{combo}` | `{ok}` — e.g. `ctrl+s`, via baton |
| `find` | `{query, role?}` | `{matches}` — text+role filter on snapshot |

## Acceptance criteria

1. `wm-desktop apps` lists at least 3 running applications by name on a typical X11 desktop.
2. `wm-desktop focus {app:"firefox"}` focuses Firefox; subsequent `xdotool getactivewindow` returns Firefox's window-id.
3. `wm-desktop read_window` on a focused gedit returns a snapshot containing the menubar items "File", "Edit", "View" with role `menuitem`.
4. `wm-desktop click {ref}` on a gedit menubar "File" ref opens the File menu.
5. `wm-desktop type {text:"hello"}` into focused gedit inserts the text.
6. `wm-desktop key {combo:"ctrl+s"}` triggers the save dialog.
7. `wm-desktop find {query:"Cancel", role:"button"}` returns the Cancel-button ref.
8. AT-SPI bus health: `wm-desktop apps` on a fresh user session with accessibility off auto-enables the bus, then succeeds on the next launch.
9. Snapshot capped at 1500 refs; on a busy file manager, `read_window` returns `truncated:true`.
10. **[live]** Real round-trip: brain chains `focus → type → key(ctrl+s)` flow end-to-end in under 15 seconds.

## Install

Requires:
- `baton` installed in `~/.local/bin/` (j0yen/baton)
- `agorabus` running (j0yen/agorabus)
- AT-SPI accessibility bus enabled (`systemctl --user enable at-spi-dbus-bus.service`)

```bash
git clone https://github.com/j0yen/wintermute-desktop
cd wintermute-desktop
cargo build --release
cp target/release/wm-desktop ~/.local/bin/
```

## License

MIT OR Apache-2.0 at your option. Copyright 2026 Joe Yen.
