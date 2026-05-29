//! AC2 (MUST): Protocol types serde-roundtrip correctly; `tool_parse` covers
//! all 7 variants plus unknown→None.
//!
//! No AT-SPI or D-Bus required — pure unit test over wire types.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "acceptance tests: assertions are the point"
)]

use wintermute_desktop::protocol::{AxNode, Command, Reply, Snapshot, Tool, cap_snapshot};

// ---------------------------------------------------------------------------
// Tool parse: all 7 known variants plus unknown→None
// ---------------------------------------------------------------------------

#[test]
fn tool_parse_all_seven_variants() {
    let pairs = [
        ("apps", Tool::Apps),
        ("focus", Tool::Focus),
        ("read_window", Tool::ReadWindow),
        ("click", Tool::Click),
        ("type", Tool::Type),
        ("key", Tool::Key),
        ("find", Tool::Find),
    ];
    for (s, expected) in pairs {
        let got = Tool::parse(s);
        assert_eq!(
            got,
            Some(expected),
            "Tool::parse(\"{s}\") should return Some({expected:?})"
        );
    }
}

#[test]
fn tool_parse_unknown_returns_none() {
    assert_eq!(Tool::parse("nonexistent_tool"), None);
    assert_eq!(Tool::parse(""), None);
    assert_eq!(Tool::parse("APPS"), None, "parse is case-sensitive");
}

// ---------------------------------------------------------------------------
// Command serde-roundtrip
// ---------------------------------------------------------------------------

#[test]
fn command_serde_roundtrip_full() {
    let original = Command {
        cmd_id: "test-cmd-1".into(),
        tool: "apps".into(),
        args: serde_json::json!({ "key": "value" }),
    };
    let encoded = serde_json::to_string(&original).unwrap();
    let decoded: Command = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn command_serde_roundtrip_without_args() {
    // `args` has `#[serde(default)]`; when absent it should default to Null.
    let json_str = r#"{"cmd_id":"c1","tool":"find"}"#;
    let cmd: Command = serde_json::from_str(json_str).unwrap();
    assert_eq!(cmd.cmd_id, "c1");
    assert_eq!(cmd.tool, "find");
    assert_eq!(cmd.args, serde_json::Value::Null);
}

// ---------------------------------------------------------------------------
// Reply serde-roundtrip (ok + err)
// ---------------------------------------------------------------------------

#[test]
fn reply_ok_serde_roundtrip() {
    let original = Reply::ok("c-ok", serde_json::json!({"apps": []}));
    let encoded = serde_json::to_string(&original).unwrap();
    let decoded: Reply = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, original);
    assert!(decoded.ok);
    assert!(decoded.error.is_none());
}

#[test]
fn reply_err_serde_roundtrip() {
    let original = Reply::err("c-err", "something went wrong");
    let encoded = serde_json::to_string(&original).unwrap();
    let decoded: Reply = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, original);
    assert!(!decoded.ok);
    assert!(decoded.result.is_none());
}

// ---------------------------------------------------------------------------
// AxNode serde — the `ref` rename must survive encode/decode
// ---------------------------------------------------------------------------

#[test]
fn axnode_serde_uses_ref_wire_key() {
    let node = AxNode {
        node_ref: "n42".into(),
        role: "button".into(),
        name: "Save".into(),
        value: String::new(),
        children_refs: vec!["n43".into()],
    };
    let json_str = serde_json::to_string(&node).unwrap();
    // The wire key must be "ref", not "node_ref".
    assert!(
        json_str.contains("\"ref\":\"n42\""),
        "wire key should be 'ref', got: {json_str}"
    );
    // Round-trip.
    let decoded: AxNode = serde_json::from_str(&json_str).unwrap();
    assert_eq!(decoded, node);
}

// ---------------------------------------------------------------------------
// Snapshot serde
// ---------------------------------------------------------------------------

#[test]
fn snapshot_serde_roundtrip() {
    let nodes = vec![AxNode {
        node_ref: "n0".into(),
        role: "window".into(),
        name: "My App".into(),
        value: String::new(),
        children_refs: vec![],
    }];
    let snap = cap_snapshot("snap-test-1", nodes);
    let encoded = serde_json::to_string(&snap).unwrap();
    let decoded: Snapshot = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, snap);
    assert!(!decoded.truncated);
    assert_eq!(decoded.snapshot_id, "snap-test-1");
}
