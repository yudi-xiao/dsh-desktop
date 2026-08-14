// Plugin marketplace transactions.
//
// Plugins are dsh profile layers under `$DSH_HOME/profiles/<name>/`. The
// transaction model mirrors oh-dsh's marketplace: mutate an isolated candidate
// profile, preview the diff, then either apply (backing up the active profile
// as `…-previous`) or discard; undo restores the previous profile.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::supervisor::resolve_dsh_cli;

const ACTIVE_PROFILE: &str = "web";
const CANDIDATE_PROFILE: &str = "web-candidate";
const PREVIOUS_PROFILE: &str = "web-previous";

#[derive(Serialize)]
pub struct PluginPreview {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub patch_diff: String,
}

fn dsh_home(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_default().join("dsh")
}

fn profile_dir(app: &AppHandle, name: &str) -> PathBuf {
    dsh_home(app).join("profiles").join(name)
}

/// Recursively copies a profile directory, skipping `node_modules` (pnpm
/// reinstalls it from the profile's lockfile/package.json).
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            if entry.file_name() == "node_modules" {
                continue;
            }
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Parses `dependencies` from a package.json into name → version-range.
fn read_deps(path: &Path) -> Result<HashMap<String, String>, String> {
    let text = read_text(path)?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
        for (k, v) in deps {
            map.insert(k.clone(), v.as_str().unwrap_or_default().to_string());
        }
    }
    Ok(map)
}

/// Minimal line diff: `+` lines only in b, `-` lines only in a.
fn diff_text(a: &str, b: &str) -> String {
    if a == b {
        return "(无变化)".to_string();
    }
    let set_a: HashSet<&str> = a.lines().collect();
    let set_b: HashSet<&str> = b.lines().collect();
    let mut out = String::new();
    for line in b.lines() {
        if !set_a.contains(line) {
            out.push_str("+ ");
            out.push_str(line);
            out.push('\n');
        }
    }
    for line in a.lines() {
        if !set_b.contains(line) {
            out.push_str("- ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Runs `dsh plugin <args>` against the pinned DSH_HOME, returning combined
/// stdout+stderr.
fn run_dsh_plugin(app: &AppHandle, args: &[&str]) -> Result<String, String> {
    let (node, bin) = resolve_dsh_cli(app).ok_or("dsh CLI not found (runtime not prepared?)")?;
    let mut cmd = Command::new(&node);
    cmd.arg(&bin).arg("plugin");
    for a in args {
        cmd.arg(a);
    }
    cmd.env("DSH_HOME", dsh_home(app));
    let output = cmd.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(format!("{stdout}\n{stderr}").trim().to_string());
    }
    Ok(format!("{stdout}\n{stderr}").trim().to_string())
}

/// Installs a plugin (any `pnpm add` spec: package name, `owner/repo`, URL…)
/// into the isolated candidate profile. Seeds the candidate from the active
/// profile on first use.
#[tauri::command]
pub fn marketplace_install(app: AppHandle, spec: String) -> Result<String, String> {
    let active = profile_dir(&app, ACTIVE_PROFILE);
    let candidate = profile_dir(&app, CANDIDATE_PROFILE);
    if !candidate.join("package.json").exists() && active.join("package.json").exists() {
        copy_dir(&active, &candidate).map_err(|e| e.to_string())?;
    }
    run_dsh_plugin(&app, &["--profile", CANDIDATE_PROFILE, "add", &spec])
}

/// Diffs the candidate profile against the active one.
#[tauri::command]
pub fn marketplace_preview(app: AppHandle) -> Result<PluginPreview, String> {
    let active = profile_dir(&app, ACTIVE_PROFILE);
    let candidate = profile_dir(&app, CANDIDATE_PROFILE);
    let active_deps = read_deps(&active.join("package.json"))?;
    let cand_deps = read_deps(&candidate.join("package.json"))?;

    let mut added = Vec::new();
    let mut removed = Vec::new();
    for (k, v) in &cand_deps {
        if !active_deps.contains_key(k) {
            added.push(format!("{k}@{v}"));
        }
    }
    for (k, v) in &active_deps {
        if !cand_deps.contains_key(k) {
            removed.push(format!("{k}@{v}"));
        }
    }

    let patch_diff = diff_text(
        &read_text(&active.join("cordis.patch.yml")).unwrap_or_default(),
        &read_text(&candidate.join("cordis.patch.yml")).unwrap_or_default(),
    );

    Ok(PluginPreview {
        added,
        removed,
        patch_diff,
    })
}

/// Applies the candidate profile: backs up the active profile as `…-previous`,
/// then promotes the candidate. The candidate directory is removed.
#[tauri::command]
pub fn marketplace_apply(app: AppHandle) -> Result<(), String> {
    let active = profile_dir(&app, ACTIVE_PROFILE);
    let candidate = profile_dir(&app, CANDIDATE_PROFILE);
    let previous = profile_dir(&app, PREVIOUS_PROFILE);

    if !candidate.join("package.json").exists() {
        return Err("没有待应用的候选 Profile".into());
    }
    if active.exists() {
        copy_dir(&active, &previous).map_err(|e| e.to_string())?;
    }
    copy_dir(&candidate, &active).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&candidate);
    Ok(())
}

/// Restores the `…-previous` profile over the active one.
#[tauri::command]
pub fn marketplace_undo(app: AppHandle) -> Result<(), String> {
    let active = profile_dir(&app, ACTIVE_PROFILE);
    let previous = profile_dir(&app, PREVIOUS_PROFILE);
    if !previous.join("package.json").exists() {
        return Err("没有可恢复的 previous Profile".into());
    }
    copy_dir(&previous, &active).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&previous);
    Ok(())
}

/// Discards the candidate profile (abandon without applying).
#[tauri::command]
pub fn marketplace_reset(app: AppHandle) -> Result<(), String> {
    let candidate = profile_dir(&app, CANDIDATE_PROFILE);
    let _ = fs::remove_dir_all(&candidate);
    Ok(())
}

/// Reports whether a candidate profile currently exists (for UI state).
#[tauri::command]
pub fn marketplace_has_candidate(app: AppHandle) -> Result<bool, String> {
    Ok(profile_dir(&app, CANDIDATE_PROFILE)
        .join("package.json")
        .exists())
}

/// Reports whether a previous profile exists (so undo is available).
#[tauri::command]
pub fn marketplace_has_previous(app: AppHandle) -> Result<bool, String> {
    Ok(profile_dir(&app, PREVIOUS_PROFILE)
        .join("package.json")
        .exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_text_marks_added_and_removed_lines() {
        let a = "line1\nline2\n";
        let b = "line1\nline3\n";
        let diff = diff_text(a, b);
        assert!(diff.contains("+ line3"), "diff was: {diff}");
        assert!(diff.contains("- line2"), "diff was: {diff}");
        assert_eq!(diff_text("same\n", "same\n"), "(无变化)");
    }

    #[test]
    fn read_deps_parses_dependencies() {
        let dir = std::env::temp_dir().join("dsh-plugins-test");
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = dir.join("package.json");
        std::fs::write(&pkg, r#"{"dependencies":{"a":"^1.0.0","b":"2.0.0"}}"#).unwrap();
        let deps = read_deps(&pkg).unwrap();
        assert_eq!(deps.get("a").map(String::as_str), Some("^1.0.0"));
        assert_eq!(deps.get("b").map(String::as_str), Some("2.0.0"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
