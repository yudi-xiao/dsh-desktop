// HarnessSupervisor: spawns the dsh web child process, waits for the readiness
// line (`dsh web: <URL>`), forwards its logs, and restarts it with exponential
// backoff on unexpected exit.
//
// Dev layout: runs the workspace `apps/runtime/dsh-web.mjs` under the system
// Node. The packaged layout (portable Node + bundled closure) is resolved in
// the resource-bundling step and layered on top of this same loop.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::{logging, platform::configure_background_command};

const READINESS_PREFIX: &str = "dsh web: ";
const INITIAL_BACKOFF_SECS: u64 = 1;
const MAX_BACKOFF_SECS: u64 = 30;

enum ChildEvent {
    Ready(String),
    Eof,
}

/// Owns the child lifecycle. Cloned handles share the same underlying state.
pub struct HarnessSupervisor {
    shutdown: Arc<AtomicBool>,
    url: Arc<Mutex<Option<String>>>,
    child_pid: Arc<Mutex<Option<u32>>>,
}

impl Default for HarnessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessSupervisor {
    pub fn new() -> Self {
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
            url: Arc::new(Mutex::new(None)),
            child_pid: Arc::new(Mutex::new(None)),
        }
    }

    /// Spawns a background thread that runs the supervise loop until shutdown.
    pub fn start(&self, app: AppHandle) {
        let shutdown = Arc::clone(&self.shutdown);
        let url = Arc::clone(&self.url);
        let child_pid = Arc::clone(&self.child_pid);
        thread::spawn(move || supervise_loop(app, shutdown, url, child_pid));
    }

    /// Requests the loop to stop and kills the current child process tree so
    /// the harness never orphans a node process.
    #[allow(dead_code)] // wired to tray Quit in the tray step
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(pid) = *self.child_pid.lock().unwrap() {
            kill_process_tree(pid);
        }
    }

    /// Last known `http://127.0.0.1:<port>` origin, once ready.
    #[allow(dead_code)] // used by the main-window integration step
    pub fn url(&self) -> Option<String> {
        self.url.lock().unwrap().clone()
    }
}

/// Applies the platform permission-mode default.
///
/// Windows has no harness confinement backend (no bwrap/Landlock/Seatbelt), so
/// dsh's default `workspace-write` mode cannot boot there. When the user has
/// not set `DSH_PERMISSION_MODE` explicitly, fall back to `danger-full-access`
/// (approval prompts disabled) and log a warning.
fn apply_permission_mode(cmd: &mut Command) {
    if cfg!(windows) && std::env::var("DSH_PERMISSION_MODE").is_err() {
        cmd.env("DSH_PERMISSION_MODE", "danger-full-access");
        logging::warn(
            "supervisor",
            "Windows has no harness confinement backend; falling back to \
             DSH_PERMISSION_MODE=danger-full-access (approval prompts disabled). \
             Set DSH_PERMISSION_MODE explicitly to override.",
        );
    }
}

/// Resolve `(program, args)` for the harness child.
///
/// Resolution order:
/// 1. `DSH_DESKTOP_DSH_CMD` (a full shell command) — for pointing the shell at
///    a checkout or a custom launcher during development.
/// 2. Dev layout — the workspace `apps/runtime/dsh-web.mjs` under system Node,
///    used when the workspace exists (a source checkout, so edits are picked up
///    live). This is preferred over the packaged layout in dev.
/// 3. Packaged layout — `resource_dir/vendor/runtime/<target>/node/` portable
///    Node running the closure extracted from `…/app.tar.gz` into the app data
///    directory. Used in a bundled app, where the compile-time workspace path
///    no longer exists on the user's machine.
fn resolve_harness_command(
    resource_dir: Option<&std::path::Path>,
    data_dir: Option<&std::path::Path>,
) -> (String, Vec<String>) {
    if let Ok(cmd) = std::env::var("DSH_DESKTOP_DSH_CMD") {
        let mut parts = cmd.split_whitespace();
        if let Some(program) = parts.next() {
            let args: Vec<String> = parts.map(str::to_string).collect();
            return (program.to_string(), args);
        }
    }

    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    let workspace_script = workspace_dsh_script();

    // Dev layout is valid only for debug binaries. A release binary retains
    // its compile-time CARGO_MANIFEST_DIR; on a developer machine that source
    // path may still exist even after installing the MSI. Treating existence
    // alone as a dev-mode signal makes the installed app run the source
    // launcher without its deployed node_modules instead of bundled runtime.
    if should_use_workspace_layout(cfg!(debug_assertions), workspace_script.exists()) {
        return (
            node_name.to_string(),
            vec![workspace_script.to_string_lossy().into_owned()],
        );
    }

    // Packaged layout: fall back to the bundled portable Node + closure.
    if let (Some(resource_dir), Some(data_dir)) = (resource_dir, data_dir) {
        let runtime = resource_dir
            .join("vendor")
            .join("runtime")
            .join(node_dist_dir());
        let node = runtime.join("node").join(node_name);
        if node.exists() {
            if let Some(script) = extract_closure(&runtime, data_dir) {
                return (
                    node.to_string_lossy().into_owned(),
                    vec![script.to_string_lossy().into_owned()],
                );
            }
        }
    }

    // Last resort: system node + the workspace launcher path (spawn will fail
    // loudly if it does not exist).
    (
        node_name.to_string(),
        vec![workspace_script.to_string_lossy().into_owned()],
    )
}

fn should_use_workspace_layout(debug_build: bool, workspace_exists: bool) -> bool {
    debug_build && workspace_exists
}

/// The workspace `apps/runtime/dsh-web.mjs` path derived from the compile-time
/// manifest location.
fn workspace_dsh_script() -> PathBuf {
    workspace_root()
        .join("apps")
        .join("runtime")
        .join("dsh-web.mjs")
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // apps/desktop/src-tauri
    manifest
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR must be <workspace>/apps/desktop/src-tauri")
        .to_path_buf()
}

/// Extracts the packaged `app.tar.gz` closure into the app data directory on
/// first launch and returns the path to `dsh-web.mjs`. The target includes an
/// archive fingerprint: repeated starts reuse it, while an application update
/// cannot accidentally keep running the previous closure.
fn extract_closure(
    runtime: &std::path::Path,
    data_dir: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let archive = runtime.join("app.tar.gz");
    if !archive.exists() {
        return None;
    }

    match extract_closure_inner(&archive, data_dir) {
        Ok(script) => Some(script),
        Err(error) => {
            logging::error(
                "supervisor",
                format!("failed to prepare bundled runtime: {error}"),
            );
            None
        }
    }
}

fn extract_closure_inner(
    archive: &std::path::Path,
    data_dir: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    let metadata = std::fs::metadata(archive)?;
    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let fingerprint = format!("{:x}-{:x}", metadata.len(), modified);
    let parent = data_dir.join("runtime").join(node_dist_dir());
    let target = parent.join(&fingerprint);
    let script = target.join("dsh-web.mjs");
    if closure_is_complete(&target) {
        return Ok(script);
    }

    std::fs::create_dir_all(&parent)?;
    remove_incomplete_runtime(&target)?;
    let staging = parent.join(format!("{fingerprint}.extracting-{}", std::process::id()));
    remove_incomplete_runtime(&staging)?;
    std::fs::create_dir_all(&staging)?;
    logging::info(
        "supervisor",
        format!("extracting runtime closure to {}", staging.display()),
    );

    // Pure-Rust extraction (flate2 + tar) avoids depending on the system `tar`,
    // whose flavor differs across platforms (GNU vs bsdtar) and misparses
    // Windows drive-letter paths.
    let file = std::fs::File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(&staging)?;

    if !closure_payload_exists(&staging) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "runtime archive is missing dsh-web.mjs or @deepseek-ai/dsh/lib/bin.js",
        ));
    }
    // The marker is written inside staging before the atomic directory rename,
    // so a crash can never make a partial target look reusable.
    std::fs::write(staging.join(".complete"), b"dsh-desktop-runtime-v1\n")?;
    std::fs::rename(&staging, &target)?;
    Ok(script)
}

fn closure_payload_exists(root: &std::path::Path) -> bool {
    root.join("dsh-web.mjs").is_file()
        && root
            .join("node_modules")
            .join("@deepseek-ai")
            .join("dsh")
            .join("lib")
            .join("bin.js")
            .is_file()
}

fn closure_is_complete(root: &std::path::Path) -> bool {
    root.join(".complete").is_file() && closure_payload_exists(root)
}

fn remove_incomplete_runtime(path: &std::path::Path) -> std::io::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// Removes only generated profile links left by a debug build of this exact
/// source checkout. They can leak into the persistent DSH_HOME and make an MSI
/// boot race against packages outside its bundled runtime. User-installed
/// plugin links point elsewhere and are deliberately preserved.
fn prune_workspace_profile_links(data_dir: &std::path::Path) -> std::io::Result<usize> {
    if cfg!(debug_assertions) {
        return Ok(0);
    }
    let cache = data_dir.join("dsh").join("profiles").join("node_modules");
    let workspace_modules = workspace_root().join("node_modules");
    let mut removed = 0;
    for entry in read_dir_if_exists(&cache)? {
        let path = entry.path();
        if remove_workspace_link(&path, &workspace_modules)? {
            removed += 1;
            continue;
        }
        // npm scopes such as @types and @deepseek-ai are real directories
        // containing one junction per package.
        if entry.file_type()?.is_dir() && entry.file_name().to_string_lossy().starts_with('@') {
            for scoped in read_dir_if_exists(&path)? {
                if remove_workspace_link(&scoped.path(), &workspace_modules)? {
                    removed += 1;
                }
            }
        }
    }
    Ok(removed)
}

fn read_dir_if_exists(path: &std::path::Path) -> std::io::Result<Vec<std::fs::DirEntry>> {
    match std::fs::read_dir(path) {
        Ok(entries) => entries.collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

fn remove_workspace_link(
    path: &std::path::Path,
    workspace_modules: &std::path::Path,
) -> std::io::Result<bool> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata_is_link(&metadata) {
        return Ok(false);
    }
    let raw_target = std::fs::read_link(path)?;
    let target = if raw_target.is_absolute() {
        raw_target
    } else {
        path.parent()
            .unwrap_or_else(|| std::path::Path::new(""))
            .join(raw_target)
    };
    if !path_is_within(&target, workspace_modules) {
        return Ok(false);
    }

    #[cfg(windows)]
    std::fs::remove_dir(path)?;
    #[cfg(not(windows))]
    std::fs::remove_file(path)?;
    logging::info(
        "supervisor",
        format!("removed stale development profile link {}", path.display()),
    );
    Ok(true)
}

fn metadata_is_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn path_is_within(path: &std::path::Path, root: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        let normalize = |value: &std::path::Path| {
            value
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('/', "\\")
                .to_ascii_lowercase()
        };
        let path = normalize(path);
        let mut root = normalize(root);
        if !root.ends_with('\\') {
            root.push('\\');
        }
        path.starts_with(&root)
    }
    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
}

/// Node distribution directory name for this build target (matches the naming
/// in `scripts/prepare-runtime.mjs`).
pub(crate) fn node_dist_dir() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "win-x64",
        ("macos", "x86_64") => "darwin-x64",
        ("macos", "aarch64") => "darwin-arm64",
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        _ => "unknown",
    }
}

/// Resolves `(node_program, dsh_bin_js)` for running any dsh CLI subcommand
/// (web, plugin, …). Used by the supervisor to boot the web profile and by the
/// plugin manager to mutate profiles.
pub fn resolve_dsh_cli(app: &AppHandle) -> Option<(String, std::path::PathBuf)> {
    let resource_dir = app.path().resource_dir().ok();
    let data_dir = app.path().app_data_dir().ok();
    let (program, args) = resolve_harness_command(resource_dir.as_deref(), data_dir.as_deref());
    // args[0] is the dsh-web.mjs launcher in the closure root; the dsh CLI is a
    // sibling under node_modules/@deepseek-ai/dsh/lib/bin.js.
    let launcher = std::path::PathBuf::from(&args[0]);
    let bin = launcher
        .parent()?
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js");
    bin.exists().then_some((program, bin))
}

fn supervise_loop(
    app: AppHandle,
    shutdown: Arc<AtomicBool>,
    url: Arc<Mutex<Option<String>>>,
    child_pid: Arc<Mutex<Option<u32>>>,
) {
    let mut backoff = INITIAL_BACKOFF_SECS;

    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        let resource_dir = app.path().resource_dir().ok();
        let data_dir = app.path().app_data_dir().ok();
        if let Some(data_dir) = &data_dir {
            match prune_workspace_profile_links(data_dir) {
                Ok(removed) if removed > 0 => logging::info(
                    "supervisor",
                    format!("removed {removed} stale development profile links"),
                ),
                Ok(_) => {}
                Err(error) => logging::warn(
                    "supervisor",
                    format!("could not repair development profile links: {error}"),
                ),
            }
        }
        let (program, args) = resolve_harness_command(resource_dir.as_deref(), data_dir.as_deref());
        logging::info("supervisor", format!("starting harness with {program}"));
        let mut cmd = Command::new(&program);
        configure_background_command(&mut cmd);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Pin DSH_HOME to the app data directory so the agent runtime and the
        // plugin manager share one profile set (and upgrades never touch it).
        if let Some(data_dir) = &data_dir {
            cmd.env("DSH_HOME", data_dir.join("dsh"));
            // The dsh-side desktop plugin reads only this sanitized snapshot;
            // OAuth credentials and raw app-server traffic stay in Rust.
            cmd.env(
                "DSH_DESKTOP_CODEX_USAGE_FILE",
                data_dir.join("codex-usage.json"),
            );
        }
        apply_permission_mode(&mut cmd);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(err) => {
                logging::error("supervisor", format!("failed to spawn {program}: {err}"));
                emit_error(&app, &format!("failed to spawn harness: {err}"));
                sleep_backoff(&shutdown, backoff);
                backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
                continue;
            }
        };
        *child_pid.lock().unwrap() = Some(child.id());
        if let Some(stderr) = child.stderr.take() {
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    logging::warn("dsh", line);
                }
            });
        }

        let ready = wait_until_ready(&mut child);

        match ready {
            Ok(Some(origin)) => {
                *url.lock().unwrap() = Some(origin.clone());
                emit_ready(&app, &origin);
                navigate_main_window(&app, &origin);
                backoff = INITIAL_BACKOFF_SECS;

                let status = child.wait();
                *url.lock().unwrap() = None;
                logging::warn("supervisor", format!("harness exited: {status:?}"));
                emit_stopped(&app);
            }
            Ok(None) => {
                // Exited before reporting ready; reap and retry.
                kill_process_tree(child.id());
                let status = child.wait();
                let message = format!(
                    "harness exited before ready ({status:?}); see the app-data logs directory"
                );
                logging::error("supervisor", &message);
                emit_error(&app, &message);
            }
            Err(err) => {
                kill_process_tree(child.id());
                let _ = child.wait();
                let message = format!("error reading harness output: {err}");
                logging::error("supervisor", &message);
                emit_error(&app, &message);
            }
        }

        *child_pid.lock().unwrap() = None;

        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        sleep_backoff(&shutdown, backoff);
        backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
    }
}

/// Spawns a reader thread for the child's stdout and waits for either the
/// readiness line or EOF. Non-readiness lines are forwarded to this process's
/// stdout as `[dsh] …` so `tauri dev` shows harness logs.
fn wait_until_ready(child: &mut Child) -> std::io::Result<Option<String>> {
    let stdout = child.stdout.take().expect("harness stdout must be piped");
    let (tx, rx): (mpsc::Sender<ChildEvent>, Receiver<ChildEvent>) = mpsc::channel();

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if let Some(url) = line.strip_prefix(READINESS_PREFIX) {
                        let _ = tx.send(ChildEvent::Ready(url.trim().to_string()));
                    } else {
                        logging::info("dsh", line);
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(ChildEvent::Eof);
    });

    for event in rx.iter() {
        match event {
            ChildEvent::Ready(url) => return Ok(Some(url)),
            ChildEvent::Eof => return Ok(None),
        }
    }
    Ok(None)
}

fn emit_ready(app: &AppHandle, url: &str) {
    logging::info("supervisor", format!("harness ready at {url}"));
    let _ = app.emit("harness-ready", url.to_string());
}

/// Points the main window at the served loopback origin once the harness is
/// ready. Until then the window shows the local connecting page.
fn navigate_main_window(app: &AppHandle, url: &str) {
    if let Some(window) = app.get_webview_window("main") {
        match tauri::Url::parse(url) {
            Ok(parsed) => {
                if let Err(err) = window.navigate(parsed) {
                    logging::error(
                        "supervisor",
                        format!("failed to navigate main window: {err}"),
                    );
                }
            }
            Err(err) => logging::error("supervisor", format!("invalid harness URL {url}: {err}")),
        }
    }
}

fn emit_stopped(app: &AppHandle) {
    let _ = app.emit("harness-stopped", ());
}

fn emit_error(app: &AppHandle, message: &str) {
    let _ = app.emit("harness-error", message.to_string());
}

/// Kills the harness process tree.
///
/// On Windows the child is a `node` wrapper that spawns the actual `dsh` CLI,
/// so `child.kill()` alone would orphan it; `taskkill /T /F` terminates the
/// whole tree. On POSIX the child was spawned in its own process group, so we
/// signal the group (SIGTERM then SIGKILL after a grace period).
fn kill_process_tree(pid: u32) {
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        configure_background_command(&mut command);
        let _ = command
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        unsafe {
            let _ = libc::kill(-(pid as i32), libc::SIGTERM);
        }
        // Grace period, then SIGKILL the group.
        thread::sleep(Duration::from_secs(1));
        unsafe {
            let _ = libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

fn sleep_backoff(shutdown: &AtomicBool, secs: u64) {
    let mut remaining = Duration::from_secs(secs);
    let tick = Duration::from_millis(200);
    while remaining > Duration::ZERO && !shutdown.load(Ordering::SeqCst) {
        let step = tick.min(remaining);
        thread::sleep(step);
        remaining -= step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_harness_command_points_at_workspace_script() {
        if std::env::var("DSH_DESKTOP_DSH_CMD").is_ok() {
            // Override present: this test only asserts the default resolution.
            return;
        }
        let (program, args) = resolve_harness_command(None, None);
        assert_eq!(
            program,
            if cfg!(windows) { "node.exe" } else { "node" },
            "dev layout must run under the system node"
        );
        assert_eq!(args.len(), 1, "dsh-web.mjs takes no default extra args");
        let script = PathBuf::from(&args[0]);
        assert!(script.exists(), "dsh-web launcher missing: {args:?}");
        assert!(script.ends_with(PathBuf::from("apps").join("runtime").join("dsh-web.mjs")));
    }

    #[test]
    fn release_build_never_selects_workspace_layout() {
        assert!(should_use_workspace_layout(true, true));
        assert!(!should_use_workspace_layout(false, true));
        assert!(!should_use_workspace_layout(true, false));
    }

    #[test]
    fn workspace_link_target_matching_respects_path_boundaries() {
        let root = workspace_root().join("node_modules");
        assert!(path_is_within(&root.join(".pnpm").join("react"), &root));
        assert!(!path_is_within(
            &workspace_root()
                .join("node_modules-elsewhere")
                .join("react"),
            &root
        ));
        assert!(!path_is_within(
            &PathBuf::from("C:/another-project/node_modules/react"),
            &root
        ));
    }

    #[test]
    fn extract_closure_unpacks_bundled_runtime() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let runtime = workspace
            .join("vendor")
            .join("runtime")
            .join(node_dist_dir());
        if !runtime.join("app.tar.gz").exists() {
            // Runtime not prepared in this environment; nothing to assert.
            return;
        }
        let data_dir = std::env::temp_dir().join("dsh-desktop-extract-test");
        let _ = std::fs::remove_dir_all(&data_dir);
        let script = extract_closure(&runtime, &data_dir).expect("closure should extract");
        assert!(script.exists(), "extracted dsh-web.mjs missing: {script:?}");
        assert!(script.ends_with("dsh-web.mjs"));
        assert!(closure_is_complete(script.parent().unwrap()));

        // Second call must be idempotent (reuse the already-extracted closure).
        let again = extract_closure(&runtime, &data_dir).expect("idempotent re-extract");
        assert_eq!(script, again);
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn readiness_prefix_parses_origin() {
        let line = "dsh web: http://127.0.0.1:1730";
        let url = line.strip_prefix(READINESS_PREFIX).unwrap().trim();
        assert_eq!(url, "http://127.0.0.1:1730");
    }

    #[test]
    fn permission_mode_fallback_applies_on_windows() {
        if !cfg!(windows) {
            return;
        }
        if std::env::var("DSH_PERMISSION_MODE").is_ok() {
            // Explicit override present; the fallback path is not exercised.
            return;
        }
        let mut cmd = Command::new("node");
        apply_permission_mode(&mut cmd);
        let mut fallback = false;
        for (key, value) in cmd.get_envs() {
            if key == "DSH_PERMISSION_MODE" {
                fallback = value == Some(std::ffi::OsStr::new("danger-full-access"));
            }
        }
        assert!(
            fallback,
            "Windows must fall back to danger-full-access when unset"
        );
    }

    /// End-to-end: spawn the real dsh-web launcher and detect readiness.
    /// Requires node on PATH and the installed dsh closure; run manually with
    /// `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn spawns_dsh_web_and_detects_readiness() {
        let (program, args) = resolve_harness_command(None, None);
        let mut child = Command::new(&program)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to spawn dsh-web (is node on PATH?)");

        let ready = wait_until_ready(&mut child).expect("reading harness stdout");
        let url = ready.expect("dsh web must report readiness before EOF");
        assert!(
            url.starts_with("http://127.0.0.1:"),
            "unexpected origin {url}"
        );

        // Kill the whole tree so the nested dsh bin.js does not orphan.
        kill_process_tree(child.id());
        let _ = child.wait();
    }
}
