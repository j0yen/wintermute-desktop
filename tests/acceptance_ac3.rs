//! AC3 (MUST): `cap_snapshot` truncates at 1500 refs and sets `truncated:true`;
//! under the cap leaves `truncated:false`.
//!
//! Pure unit test, no AT-SPI required.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "acceptance tests: assertions are the point"
)]

use wintermute_desktop::protocol::{AxNode, SNAPSHOT_CAP, cap_snapshot};

fn make_node(i: usize) -> AxNode {
    AxNode {
        node_ref: format!("n{i}"),
        role: "label".into(),
        name: format!("node-{i}"),
        value: String::new(),
        children_refs: vec![],
    }
}

#[test]
fn cap_snapshot_over_cap_truncates_and_flags() {
    let n = SNAPSHOT_CAP + 300;
    let nodes: Vec<AxNode> = (0..n).map(make_node).collect();
    let snap = cap_snapshot("s1", nodes);
    assert_eq!(snap.nodes.len(), SNAPSHOT_CAP, "must cap to SNAPSHOT_CAP");
    assert!(snap.truncated, "truncated must be true when over cap");
}

#[test]
fn cap_snapshot_under_cap_not_truncated() {
    let n = SNAPSHOT_CAP - 1;
    let nodes: Vec<AxNode> = (0..n).map(make_node).collect();
    let snap = cap_snapshot("s2", nodes);
    assert_eq!(snap.nodes.len(), n);
    assert!(!snap.truncated, "truncated must be false under cap");
}

#[test]
fn cap_snapshot_exactly_at_cap_not_truncated() {
    let nodes: Vec<AxNode> = (0..SNAPSHOT_CAP).map(make_node).collect();
    let snap = cap_snapshot("s3", nodes);
    assert_eq!(snap.nodes.len(), SNAPSHOT_CAP);
    assert!(!snap.truncated, "exactly at cap must not truncate");
}

#[test]
fn cap_snapshot_empty_not_truncated() {
    let snap = cap_snapshot("s4", vec![]);
    assert!(snap.nodes.is_empty());
    assert!(!snap.truncated);
}

#[test]
fn cap_snapshot_preserves_snapshot_id() {
    let snap = cap_snapshot("my-uuid-1234", vec![]);
    assert_eq!(snap.snapshot_id, "my-uuid-1234");
}
