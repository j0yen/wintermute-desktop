//! AT-SPI accessibility tree → [`AxNode`] conversion.
//!
//! This module walks the live AT-SPI accessible-object tree for all running
//! applications and produces a flat [`Vec<AxNode>`] that `cap_snapshot` can
//! cap at [`SNAPSHOT_CAP`] refs.
//!
//! The conversion is on-demand (no persistent subscription); the daemon
//! calls [`build_snapshot`] and [`list_apps`] in response to tool commands.

use anyhow::{Context as _, Result};
use atspi_connection::AccessibilityConnection;
use atspi_proxies::accessible::AccessibleProxy;
use uuid::Uuid;
use zbus::proxy::CacheProperties;

use crate::protocol::{AxNode, Role, Snapshot, SNAPSHOT_CAP, cap_snapshot};

/// Build a flat [`Snapshot`] of all visible windows by walking the AT-SPI
/// accessible-object tree from the registry root.
///
/// Returns a snapshot whose `nodes.len() <= SNAPSHOT_CAP`; `truncated` is
/// `true` when the live tree exceeded the cap.
///
/// # Errors
///
/// Returns `Err` if the AT-SPI connection cannot reach the registry or the
/// tree walk fails with a D-Bus error.
#[allow(clippy::future_not_send)]
pub async fn build_snapshot(conn: &AccessibilityConnection) -> Result<Snapshot> {
    let nodes = collect_nodes(conn).await?;
    let id = Uuid::new_v4().to_string();
    Ok(cap_snapshot(id, nodes))
}

/// Collect up to `SNAPSHOT_CAP + 1` nodes from the AT-SPI tree using an
/// iterative BFS queue.
///
/// We cap the walk at `SNAPSHOT_CAP + 1` so that `cap_snapshot` can set
/// `truncated` correctly without traversing enormous trees in full.
#[allow(clippy::future_not_send)]
async fn collect_nodes(conn: &AccessibilityConnection) -> Result<Vec<AxNode>> {
    let root = conn
        .root_accessible_on_registry()
        .await
        .context("getting AT-SPI registry root")?;

    // The registry root's children are the top-level application objects.
    let apps = root
        .get_children()
        .await
        .context("listing AT-SPI applications")?;

    let mut nodes: Vec<AxNode> = Vec::new();
    let mut counter: usize = 0;

    // BFS queue: (dest, path, parent_index_in_nodes or None for roots)
    let mut queue: std::collections::VecDeque<(String, String, Option<usize>)> =
        std::collections::VecDeque::new();

    for app_ref in apps {
        let Some(dest) = app_ref.name_as_str() else {
            continue;
        };
        queue.push_back((dest.to_string(), app_ref.path_as_str().to_string(), None));
    }

    while let Some((dest, path, parent_idx)) = queue.pop_front() {
        if counter > SNAPSHOT_CAP {
            break;
        }

        let Ok(proxy) = make_proxy(conn.connection(), &dest, &path).await else {
            // Increment counter so pre-registered child_refs stay consistent,
            // but only if this node was pre-registered by a parent.
            if parent_idx.is_some() {
                counter += 1;
            }
            continue;
        };

        let node_ref = format!("n{counter}");
        counter += 1;

        let role_str = proxy
            .get_role_name()
            .await
            .unwrap_or_else(|_| "other".to_string());
        let name = proxy.name().await.unwrap_or_default();
        let role = Role::from_atspi(&role_str);

        let my_index = nodes.len();

        // If we have a parent, register this node's ref in the parent's children_refs.
        if let Some(pidx) = parent_idx {
            if let Some(parent_node) = nodes.get_mut(pidx) {
                parent_node.children_refs.push(node_ref.clone());
            }
        }

        nodes.push(AxNode {
            node_ref,
            role: role.to_string(),
            name,
            value: String::new(),
            children_refs: Vec::new(),
        });

        // Enqueue children if we're not already at cap.
        if counter <= SNAPSHOT_CAP {
            let children = proxy.get_children().await.unwrap_or_default();
            for child_ref in children {
                if counter > SNAPSHOT_CAP {
                    break;
                }
                let Some(cdest) = child_ref.name_as_str() else {
                    continue;
                };
                queue.push_back((
                    cdest.to_string(),
                    child_ref.path_as_str().to_string(),
                    Some(my_index),
                ));
            }
        }
    }

    Ok(nodes)
}

/// Build an [`AccessibleProxy`] for the given D-Bus destination + object path.
#[allow(clippy::future_not_send)]
async fn make_proxy<'a>(
    conn: &'a zbus::Connection,
    dest: &'a str,
    path: &'a str,
) -> Result<AccessibleProxy<'a>> {
    AccessibleProxy::builder(conn)
        .destination(dest)
        .context("invalid destination")?
        .path(path)
        .context("invalid path")?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .context("building AccessibleProxy")
}

/// List all running AT-SPI applications.
///
/// Returns a JSON array of `{name, pid, window_ids}` entries. `pid` is
/// best-effort (0 when the application does not expose it via AT-SPI).
///
/// # Errors
///
/// Returns `Err` if the AT-SPI connection fails or registry root lookup fails.
#[allow(clippy::future_not_send)]
pub async fn list_apps(conn: &AccessibilityConnection) -> Result<Vec<serde_json::Value>> {
    use serde_json::json;

    let root = conn
        .root_accessible_on_registry()
        .await
        .context("getting AT-SPI registry root for apps")?;

    let apps = root
        .get_children()
        .await
        .context("listing AT-SPI apps")?;

    let mut result = Vec::new();
    for app_ref in apps {
        let Some(dest_str) = app_ref.name_as_str() else {
            continue;
        };
        let dest = dest_str.to_string();
        let path = app_ref.path_as_str().to_string();

        let Ok(proxy) = make_proxy(conn.connection(), &dest, &path).await else {
            continue;
        };

        let name = proxy.name().await.unwrap_or_else(|_| dest.clone());

        result.push(json!({
            "name": name,
            "pid": 0,
            "window_ids": [],
        }));
    }

    Ok(result)
}
