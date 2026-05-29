//! Wire types for the `wm-desktop` agorabus interface and the pure
//! accessibility-snapshot data model.
//!
//! Everything in this module is plain data + pure functions: no AT-SPI,
//! no I/O. That keeps the command/reply envelopes and the snapshot-cap
//! logic (PRD AC9 / intent-card AC3) unit-testable without a D-Bus session.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum number of nodes returned in a single `read_window` snapshot.
///
/// PRD §2.3 / intent-card AC3: a busy file manager can carry thousands of
/// nodes, which would blow the brain's context window. We cap the returned
/// snapshot at this many nodes and set [`Snapshot::truncated`] so the brain
/// falls back to `find` rather than paging the whole tree.
pub const SNAPSHOT_CAP: usize = 1500;

/// Normalized role vocabulary shared across desktop surfaces.
///
/// AT-SPI roles are mapped to this small vocabulary so the brain has one
/// consistent mental model across `wm-browser` and `wm-desktop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// An activatable button.
    Button,
    /// An editable text field.
    Textfield,
    /// A static text label.
    Label,
    /// A menu item (including toolbar buttons that behave like menu items).
    Menuitem,
    /// A top-level window or dialog.
    Window,
    /// A container (panel, group, frame, box, etc.).
    Container,
    /// Any role not covered by the vocabulary above.
    Other,
}

impl Role {
    /// Map an AT-SPI role string to our vocabulary.
    ///
    /// The mapping is deliberately conservative: prefer `Container` over
    /// `Other` for grouping roles (panel, scroll-pane, …) so the brain
    /// can navigate structure without being overwhelmed by `Other` noise.
    #[must_use]
    pub fn from_atspi(role: &str) -> Self {
        match role {
            "push button" | "toggle button" | "check box" | "radio button"
            | "spin button" | "combo box" => Self::Button,
            "text" | "entry" | "editable text" | "password text" => Self::Textfield,
            "label" | "static text" => Self::Label,
            "menu item" | "check menu item" | "radio menu item"
            | "tearoff menu item" | "tool bar" => Self::Menuitem,
            "frame" | "dialog" | "window" | "alert" | "file chooser" => Self::Window,
            "panel" | "scroll pane" | "filler" | "page tab list" | "page tab"
            | "table" | "tree" | "tree table" | "list" | "section"
            | "document frame" | "internal frame" | "layered pane"
            | "glass pane" | "root pane" | "split pane" | "viewport"
            | "drawing area" | "canvas" => Self::Container,
            _ => Self::Other,
        }
    }

    /// The canonical lowercase wire string for this role.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Textfield => "textfield",
            Self::Label => "label",
            Self::Menuitem => "menuitem",
            Self::Window => "window",
            Self::Container => "container",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The set of desktop actions the daemon understands.
///
/// Mirrors the PRD §2.2 tool table. `parse` maps the wire `tool` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// List running applications.
    Apps,
    /// Focus a window or application.
    Focus,
    /// Read the AT-SPI tree of the focused window.
    ReadWindow,
    /// Click an element by snapshot ref.
    Click,
    /// Type text into the focused window via baton.
    Type,
    /// Send a key combo via baton.
    Key,
    /// Filter the latest snapshot by text + optional role.
    Find,
}

impl Tool {
    /// Parse the wire `tool` string into a [`Tool`].
    ///
    /// Returns `None` for an unknown tool name so the daemon can reply
    /// with a structured error instead of panicking.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "apps" => Some(Self::Apps),
            "focus" => Some(Self::Focus),
            "read_window" => Some(Self::ReadWindow),
            "click" => Some(Self::Click),
            "type" => Some(Self::Type),
            "key" => Some(Self::Key),
            "find" => Some(Self::Find),
            _ => None,
        }
    }

    /// The canonical wire name for this tool.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Apps => "apps",
            Self::Focus => "focus",
            Self::ReadWindow => "read_window",
            Self::Click => "click",
            Self::Type => "type",
            Self::Key => "key",
            Self::Find => "find",
        }
    }
}

/// Incoming command envelope on topic `wm.desktop.cmd`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    /// Opaque correlation id echoed back on the reply.
    pub cmd_id: String,
    /// Tool name (`apps`, `focus`, …). Validated via [`Tool::parse`].
    pub tool: String,
    /// Tool-specific arguments. Shape depends on `tool`.
    #[serde(default)]
    pub args: Value,
}

/// Outgoing reply envelope on topic `wm.desktop.reply`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reply {
    /// Echoed `cmd_id` from the originating [`Command`].
    pub cmd_id: String,
    /// Whether the tool ran successfully.
    pub ok: bool,
    /// Tool result payload on success; `null` on failure.
    pub result: Option<Value>,
    /// Human-readable error on failure; `null` on success.
    pub error: Option<String>,
}

impl Reply {
    /// Build a success reply for `cmd_id` carrying `result`.
    #[must_use]
    pub fn ok(cmd_id: impl Into<String>, result: Value) -> Self {
        Self {
            cmd_id: cmd_id.into(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    /// Build an error reply for `cmd_id` carrying `error`.
    #[must_use]
    pub fn err(cmd_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            cmd_id: cmd_id.into(),
            ok: false,
            result: None,
            error: Some(error.into()),
        }
    }
}

/// A single accessibility node in a flat snapshot.
///
/// `ref` is an opaque per-snapshot id (`"n0"`, `"n1"`, …); the brain
/// passes it back unchanged to `click`/`type`. Internally the daemon
/// keeps a `ref -> accessible-object` map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AxNode {
    /// Opaque per-snapshot stable id.
    #[serde(rename = "ref")]
    pub node_ref: String,
    /// Normalized role string (from the [`Role`] vocabulary).
    pub role: String,
    /// Accessible name (visible text / AT-SPI accessible-name).
    pub name: String,
    /// Form value where applicable (text field contents); empty otherwise.
    pub value: String,
    /// Refs of this node's children within the same snapshot.
    pub children_refs: Vec<String>,
}

/// A capped, flat accessibility snapshot of a window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::struct_field_names, reason = "snapshot_id is a wire-protocol field name; renaming would break serde shape")]
pub struct Snapshot {
    /// Opaque UUID identifying this snapshot version.
    pub snapshot_id: String,
    /// The (possibly capped) node list.
    pub nodes: Vec<AxNode>,
    /// `true` when the original tree exceeded [`SNAPSHOT_CAP`] and was
    /// truncated. The brain should switch to `find` when this is set.
    pub truncated: bool,
}

/// Cap a node list at [`SNAPSHOT_CAP`], reporting whether truncation
/// occurred. Pure function — the backbone of intent-card AC3's unit test.
///
/// Returns a [`Snapshot`] whose `nodes.len() <= SNAPSHOT_CAP` and whose
/// `truncated` flag is `true` iff the input exceeded the cap.
#[must_use]
pub fn cap_snapshot(snapshot_id: impl Into<String>, mut nodes: Vec<AxNode>) -> Snapshot {
    let truncated = nodes.len() > SNAPSHOT_CAP;
    if truncated {
        nodes.truncate(SNAPSHOT_CAP);
    }
    Snapshot {
        snapshot_id: snapshot_id.into(),
        nodes,
        truncated,
    }
}

/// Case-insensitive substring match of `query` against a node's role or name.
///
/// If `role_filter` is `Some`, only nodes whose role string equals the filter
/// (case-insensitive) are considered. Pure helper shared by the live `find`
/// tool and its tests.
#[must_use]
pub fn node_matches(node: &AxNode, query: &str, role_filter: Option<&str>) -> bool {
    if let Some(rf) = role_filter {
        if !node.role.eq_ignore_ascii_case(rf) {
            return false;
        }
    }
    let q = query.to_lowercase();
    node.name.to_lowercase().contains(&q) || node.role.to_lowercase().contains(&q)
}

/// Filter `nodes` to those matching `query` and optional `role_filter`,
/// mapping each to the `{ref, role, name}` shape the `find` tool returns.
#[must_use]
pub fn find_matches(nodes: &[AxNode], query: &str, role_filter: Option<&str>) -> Vec<Value> {
    nodes
        .iter()
        .filter(|n| node_matches(n, query, role_filter))
        .map(|n| {
            serde_json::json!({
                "ref": n.node_ref,
                "role": n.role,
                "name": n.name,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: assertions are the point, not production code"
)]
mod tests {
    use super::*;

    fn node(i: usize, role: &str, name: &str) -> AxNode {
        AxNode {
            node_ref: format!("n{i}"),
            role: role.to_string(),
            name: name.to_string(),
            value: String::new(),
            children_refs: vec![],
        }
    }

    #[test]
    fn tool_parse_roundtrips_all_variants() {
        for t in [
            Tool::Apps,
            Tool::Focus,
            Tool::ReadWindow,
            Tool::Click,
            Tool::Type,
            Tool::Key,
            Tool::Find,
        ] {
            assert_eq!(Tool::parse(t.as_str()), Some(t));
        }
        assert_eq!(Tool::parse("unknown_tool"), None);
    }

    #[test]
    fn command_serde_roundtrip() {
        let cmd = Command {
            cmd_id: "c-1".into(),
            tool: "apps".into(),
            args: serde_json::json!({}),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cmd);
    }

    #[test]
    fn command_defaults_args_when_absent() {
        let back: Command = serde_json::from_str(r#"{"cmd_id":"c","tool":"apps"}"#).unwrap();
        assert_eq!(back.args, Value::Null);
    }

    #[test]
    fn reply_ok_and_err_shapes() {
        let ok = Reply::ok("c-2", serde_json::json!({"apps": []}));
        assert!(ok.ok);
        assert!(ok.error.is_none());

        let err = Reply::err("c-3", "boom");
        assert!(!err.ok);
        assert!(err.result.is_none());
        assert_eq!(err.error.unwrap(), "boom");
    }

    #[test]
    fn axnode_serde_uses_ref_key() {
        let n = node(0, "button", "Save");
        let json = serde_json::to_string(&n).unwrap();
        assert!(json.contains("\"ref\":\"n0\""), "json was {json}");
        let back: AxNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn cap_snapshot_truncates_over_cap() {
        let nodes: Vec<AxNode> = (0..SNAPSHOT_CAP + 200)
            .map(|i| node(i, "label", "x"))
            .collect();
        let snap = cap_snapshot("snap-1", nodes);
        assert_eq!(snap.nodes.len(), SNAPSHOT_CAP);
        assert!(snap.truncated);
    }

    #[test]
    fn cap_snapshot_under_cap_not_truncated() {
        let nodes: Vec<AxNode> = (0..SNAPSHOT_CAP - 1)
            .map(|i| node(i, "label", "x"))
            .collect();
        let snap = cap_snapshot("snap-2", nodes);
        assert_eq!(snap.nodes.len(), SNAPSHOT_CAP - 1);
        assert!(!snap.truncated);
    }

    #[test]
    fn cap_snapshot_exactly_at_cap_not_truncated() {
        let nodes: Vec<AxNode> = (0..SNAPSHOT_CAP).map(|i| node(i, "label", "x")).collect();
        let snap = cap_snapshot("snap-3", nodes);
        assert_eq!(snap.nodes.len(), SNAPSHOT_CAP);
        assert!(!snap.truncated);
    }

    #[test]
    fn role_from_atspi_maps_known_roles() {
        assert_eq!(Role::from_atspi("push button"), Role::Button);
        assert_eq!(Role::from_atspi("text"), Role::Textfield);
        assert_eq!(Role::from_atspi("label"), Role::Label);
        assert_eq!(Role::from_atspi("menu item"), Role::Menuitem);
        assert_eq!(Role::from_atspi("frame"), Role::Window);
        assert_eq!(Role::from_atspi("panel"), Role::Container);
        assert_eq!(Role::from_atspi("unknown_role_xyz"), Role::Other);
    }

    #[test]
    fn node_matches_case_insensitive_over_role_and_name() {
        let n = node(0, "menuitem", "File");
        assert!(node_matches(&n, "file", None));
        assert!(node_matches(&n, "FILE", None));
        assert!(node_matches(&n, "menuitem", None));
        assert!(!node_matches(&n, "cancel", None));
    }

    #[test]
    fn node_matches_role_filter_excludes_wrong_role() {
        let n = node(0, "button", "Cancel");
        assert!(node_matches(&n, "cancel", Some("button")));
        assert!(!node_matches(&n, "cancel", Some("menuitem")));
    }

    #[test]
    fn find_matches_projects_ref_role_name() {
        let nodes = vec![
            node(0, "menuitem", "File"),
            node(1, "menuitem", "Edit"),
            node(2, "button", "Cancel"),
        ];
        let hits = find_matches(&nodes, "cancel", Some("button"));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["ref"], "n2");
        assert_eq!(hits[0]["role"], "button");
        assert_eq!(hits[0]["name"], "Cancel");
    }

    #[test]
    fn find_matches_no_role_filter() {
        let nodes = vec![
            node(0, "menuitem", "File"),
            node(1, "label", "File size"),
        ];
        let hits = find_matches(&nodes, "file", None);
        assert_eq!(hits.len(), 2);
    }
}
