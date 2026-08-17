//! Project board: read the dsh workspace registry and project it as cards.
//!
//! The dsh runtime persists its workspace registry to
//! `$DSH_HOME/storages/workspace.json` (a cosmokit storage unit). The schema
//! (v2) is stable at the durable boundary:
//!
//! ```json
//! {
//!   "unit": { "name": "workspace", "version": 2 },
//!   "global": { "initialized": true, "workspaceIds": ["…"], "archivedSessionIds": ["…"] },
//!   "tables": { "workspaces": { "<id>": { "path", "title", "sessionIds", "createdAt", "updatedAt" } } }
//! }
//! ```
//!
//! We read that file directly rather than talking to the dsh web `/api` RPC
//! gateway: the durable shape is the stable contract, and reading a JSON file
//! avoids depending on the (developer-preview) wire protocol.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};

/// One workspace card shown on the project board.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCard {
    pub id: String,
    pub title: String,
    pub path: String,
    pub session_count: usize,
    pub archived_session_count: usize,
    /// `active` | `empty` | `archived`
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// The dsh home directory pinned by the shell (same as the plugin market).
fn dsh_home(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_default().join("dsh")
}

/// Lists all workspaces in the registry's durable display order as cards.
/// An empty registry (or a missing storage file, e.g. dsh never ran) yields
/// an empty list rather than an error.
#[tauri::command]
pub fn board_list_workspaces(app: AppHandle) -> Result<Vec<WorkspaceCard>, String> {
    let file = dsh_home(&app).join("storages").join("workspace.json");
    if !file.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(parse_workspaces(&json))
}

/// Opens a workspace directory in the system file manager.
#[tauri::command]
pub fn board_open_path(path: String) -> Result<(), String> {
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

/// Focuses the main window (the dsh web UI) so the user can open the project
/// there. Works without the dsh RPC gateway, which does not expose workspace
/// switching over HTTP in the current developer-preview version.
#[tauri::command]
pub fn board_focus_main(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

/// Projects the workspace storage unit into an ordered card list.
fn parse_workspaces(json: &serde_json::Value) -> Vec<WorkspaceCard> {
    let archived: HashSet<&str> = json
        .pointer("/global/archivedSessionIds")
        .and_then(serde_json::Value::as_array)
        .map(|arr| arr.iter().filter_map(serde_json::Value::as_str).collect())
        .unwrap_or_default();

    let table = json.pointer("/tables/workspaces");

    // `global.workspaceIds` is the authoritative display order; fall back to
    // the table's key order for registries written before that field existed.
    let ids: Vec<String> = json
        .pointer("/global/workspaceIds")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .or_else(|| {
            table
                .and_then(serde_json::Value::as_object)
                .map(|obj| obj.keys().cloned().collect())
        })
        .unwrap_or_default();

    ids.into_iter()
        .filter_map(|id| {
            let record = table.and_then(|t| t.get(&id))?;
            let session_ids: Vec<&str> = record
                .get("sessionIds")
                .and_then(serde_json::Value::as_array)
                .map(|arr| arr.iter().filter_map(serde_json::Value::as_str).collect())
                .unwrap_or_default();

            let session_count = session_ids.len();
            let archived_session_count = session_ids.iter().filter(|s| archived.contains(*s)).count();
            let status = if session_count == 0 {
                "empty"
            } else if archived_session_count == session_count {
                "archived"
            } else {
                "active"
            };

            Some(WorkspaceCard {
                id,
                title: record
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                path: record
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                session_count,
                archived_session_count,
                status: status.to_string(),
                created_at: record
                    .get("createdAt")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                updated_at: record
                    .get("updatedAt")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> serde_json::Value {
        serde_json::json!({
            "unit": { "name": "workspace", "version": 2 },
            "global": {
                "initialized": true,
                "workspaceIds": ["ws-a", "ws-b", "ws-c"],
                "archivedSessionIds": ["s2"]
            },
            "tables": {
                "workspaces": {
                    "ws-a": {
                        "path": "C:/proj/alpha",
                        "title": "alpha",
                        "sessionIds": ["s1", "s2"],
                        "createdAt": "2026-08-01T00:00:00Z",
                        "updatedAt": "2026-08-14T00:00:00Z"
                    },
                    "ws-b": {
                        "path": "C:/proj/beta",
                        "title": "beta",
                        "sessionIds": [],
                        "createdAt": "2026-08-02T00:00:00Z",
                        "updatedAt": "2026-08-03T00:00:00Z"
                    },
                    "ws-c": {
                        "path": "C:/proj/gamma",
                        "title": "gamma",
                        "sessionIds": ["s3"],
                        "createdAt": "2026-08-03T00:00:00Z",
                        "updatedAt": "2026-08-04T00:00:00Z"
                    }
                }
            }
        })
    }

    #[test]
    fn parses_workspaces_in_display_order() {
        let cards = parse_workspaces(&sample());
        assert_eq!(cards.len(), 3);
        assert_eq!(
            cards.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            vec!["ws-a", "ws-b", "ws-c"]
        );
    }

    #[test]
    fn derives_status_from_session_accounting() {
        let cards = parse_workspaces(&sample());
        let by_id = |id: &str| cards.iter().find(|c| c.id == id).unwrap();

        // ws-a: two sessions, one archived → still active.
        let a = by_id("ws-a");
        assert_eq!(a.session_count, 2);
        assert_eq!(a.archived_session_count, 1);
        assert_eq!(a.status, "active");

        // ws-b: no sessions → empty.
        assert_eq!(by_id("ws-b").status, "empty");

        // ws-c: one non-archived session → active.
        assert_eq!(by_id("ws-c").status, "active");
    }

    #[test]
    fn missing_storage_parses_to_empty() {
        assert!(parse_workspaces(&serde_json::json!({})).is_empty());
    }
}
