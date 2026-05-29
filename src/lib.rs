//! `wintermute-desktop` — AT-SPI tree reading + keystroke injection for the
//! wintermute brain.
//!
//! The crate exposes:
//! - [`protocol`]: pure wire types (`Command`, `Reply`, `Tool`, `AxNode`,
//!   `Snapshot`), the snapshot-cap function, and the `find_matches` helper.
//! - [`actions`]: thin async wrappers around `baton type/key/focus`.
//! - [`snapshot`]: AT-SPI accessibility tree → `AxNode` conversion.
//! - [`daemon`]: agorabus subscribe loop and tool dispatcher.
//!
//! The `wm-desktop` binary (`src/main.rs`) exposes two surfaces:
//! - `wm-desktop daemon` — long-running agorabus subscriber.
//! - `wm-desktop <tool> [args]` — one-shot CLI mode, prints JSON reply.

#![warn(missing_docs)]

pub mod actions;
pub mod daemon;
pub mod protocol;
pub mod snapshot;
