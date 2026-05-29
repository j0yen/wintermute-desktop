# AUTOBUILDER_PROGRAM — wintermute-desktop

## Role

You are the edit-agent for `wintermute-desktop`. Your only write surface is `src/`. You implement the `wm-desktop` daemon and its protocol types so each acceptance test in `tests/acceptance_*.rs` reaches green.

## Intent

Root motivation: Give the wintermute brain a read+act surface over the entire X11 desktop using AT-SPI accessibility semantics for reading and xdotool/baton for acting.

Binary: `wm-desktop`. Mirrors `wm-browser`'s shape (agorabus pub/sub, snapshot-with-refs).

## Unfakeable metric

`acceptance_tests_passing_count` — maximize. Read from `target/autobuilder/metrics.json.scalars.acceptance_tests_passing_count` after every `scripts/run-metrics.sh` run.

## Files you may edit

- `src/**/*.rs` — all implementation
- `Cargo.toml` — only the `[dependencies]` section (add/update deps); do NOT touch lints, edition, msrv, or profile settings

## Files you must NOT edit

- `scripts/` — harness scripts (read-only)
- `tests/acceptance_*.rs` — the test harness body (you may NOT modify these)
- `tests/proptest_invariants.rs` — read-only
- `clippy.toml`, `deny.toml`, `rust-toolchain.toml` — scaffold-owned
- `agent/` — orchestrator-owned (except you may read intent-card.json)

## Advance criteria

A change advances (is NOT reverted) when ALL of:
1. `cargo check --workspace` exits 0
2. `cargo clippy --workspace -- -D warnings` exits 0
3. `cargo test --workspace` exits 0
4. `acceptance_tests_passing_count` ≥ prior iteration's count (no regression)
5. No BLOCKING findings from `scripts/audit.sh`

## Key design decisions

- **Protocol**: `wm.desktop.cmd` input topic, `wm.desktop.reply` output topic
- **Snapshot cap**: 1500 nodes (SNAPSHOT_CAP constant in `src/protocol.rs`)
- **Role vocabulary**: `button | textfield | label | menuitem | window | container | other`
- **baton delegation**: `type` and `key` tools shell out to `baton type <text>` / `baton key <combo>`
- **AT-SPI**: use `atspi-connection` + `atspi-common`; no raw unsafe zbus calls
- **Tokio runtime**: multi-thread, standard tokio::main

## Module layout suggestion

```
src/
  lib.rs        — re-exports protocol, snapshot, daemon
  main.rs       — clap CLI, tokio main
  protocol.rs   — Command, Reply, Tool, AxNode, Snapshot, cap_snapshot, find_matches
  snapshot.rs   — AT-SPI tree → AxNode conversion, snapshot_id generation
  daemon.rs     — agorabus subscribe loop, tool dispatch
  actions.rs    — baton subprocess delegation (type, key, focus)
```
