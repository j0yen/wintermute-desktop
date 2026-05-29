//! AC4 (MUST): `find_matches` filters by case-insensitive text + optional role predicate;
//! returns `[{ref, role, name}]` shape matching the protocol spec.
//!
//! Pure unit test, no AT-SPI required.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "acceptance tests: assertions are the point"
)]

use wintermute_desktop::protocol::{AxNode, find_matches};

fn node(node_ref: &str, role: &str, name: &str) -> AxNode {
    AxNode {
        node_ref: node_ref.to_string(),
        role: role.to_string(),
        name: name.to_string(),
        value: String::new(),
        children_refs: vec![],
    }
}

fn sample_nodes() -> Vec<AxNode> {
    vec![
        node("n0", "menuitem", "File"),
        node("n1", "menuitem", "Edit"),
        node("n2", "menuitem", "View"),
        node("n3", "button", "Cancel"),
        node("n4", "button", "OK"),
        node("n5", "label", "File size: 1 KB"),
    ]
}

#[test]
fn find_matches_case_insensitive_name_match() {
    let nodes = sample_nodes();
    let hits = find_matches(&nodes, "file", None);
    // Should match "File" (menuitem) and "File size: 1 KB" (label)
    assert_eq!(hits.len(), 2, "expected 2 hits for 'file', got: {hits:?}");
    for hit in &hits {
        let name = hit["name"].as_str().unwrap();
        assert!(
            name.to_lowercase().contains("file"),
            "hit name '{name}' should contain 'file'"
        );
    }
}

#[test]
fn find_matches_with_role_filter() {
    let nodes = sample_nodes();
    // Only menuitem role matching "edit"
    let hits = find_matches(&nodes, "edit", Some("menuitem"));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["ref"], "n1");
    assert_eq!(hits[0]["role"], "menuitem");
    assert_eq!(hits[0]["name"], "Edit");
}

#[test]
fn find_matches_role_filter_excludes_other_roles() {
    let nodes = sample_nodes();
    // "cancel" exists as a button, not a menuitem
    let hits = find_matches(&nodes, "cancel", Some("menuitem"));
    assert_eq!(hits.len(), 0, "cancel is a button, not a menuitem");
}

#[test]
fn find_matches_role_filter_case_insensitive() {
    let nodes = sample_nodes();
    let hits_lower = find_matches(&nodes, "ok", Some("button"));
    let hits_upper = find_matches(&nodes, "OK", Some("BUTTON"));
    assert_eq!(hits_lower.len(), 1);
    assert_eq!(hits_upper.len(), 1);
    assert_eq!(hits_lower[0]["ref"], hits_upper[0]["ref"]);
}

#[test]
fn find_matches_result_shape_has_ref_role_name() {
    let nodes = sample_nodes();
    let hits = find_matches(&nodes, "cancel", Some("button"));
    assert_eq!(hits.len(), 1);
    let h = &hits[0];
    // All three fields must be present as strings
    assert!(h["ref"].is_string(), "ref must be a string");
    assert!(h["role"].is_string(), "role must be a string");
    assert!(h["name"].is_string(), "name must be a string");
}

#[test]
fn find_matches_empty_nodes_returns_empty() {
    let hits = find_matches(&[], "anything", None);
    assert!(hits.is_empty());
}

#[test]
fn find_matches_no_match_returns_empty() {
    let nodes = sample_nodes();
    let hits = find_matches(&nodes, "nonexistent_xyz_999", None);
    assert!(hits.is_empty());
}
