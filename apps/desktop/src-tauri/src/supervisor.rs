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
        eprintln!(
            "[supervisor] Windows has no harness confinement backend; \
             falling back to DSH_PERMISSION_MODE=danger-full-access \
             (approval prompts disabled). Set DSH_PERMISSION_MODE explicitly to override."
        );
    }
}

/// Resolve `(program, args)` for the harness child.
///
/// Resolution order:
/// 1. `DSH_DESKTOP_DSH_CMD` (a full shell command) — for pointing the shell at
///    a checkout or a custom launcher during development.
/// 2. Packaged layout — `resource_dir/vendor/runtime/<target>/node/` portable
///    Node running the closure extracted from `…/app.tar.gz` into the app data
///    directory (Node handles long paths there; the archive keeps every bundled
///    path short enough for the NSIS installer).
/// 3. Dev layout — the workspace `apps/runtime/dsh-web.mjs` under system Node.
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

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // apps/desktop/src-tauri
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR must be <workspace>/apps/desktop/src-tauri");
    let script = workspace
        .join("apps")
        .join("runtime")
        .join("dsh-web.mjs");

    (node_name.to_string(), vec![script.to_string_lossy().into_owned()])
}

/// Extracts the packaged `app.tar.gz` closure into the app data directory on
/// first launch and returns the path to `dsh-web.mjs`. Extraction is idempotent:
/// an already-extracted closure is reused.
fn extract_closure(runtime: &std::path::Path, data_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let archive = runtime.join("app.tar.gz");
    if !archive.exists() {
        return None;
    }

    let target = data_dir.join("runtime").join(node_dist_dir());
    let script = target.join("dsh-web.mjs");
    if script.exists() {
        return Some(script);
    }

    std::fs::create_dir_all(&target).ok()?;
    eprintln!("[supervisor] extracting runtime closure to {}", target.display());

    // Pure-Rust extraction (flate2 + tar) avoids depending on the system `tar`,
    // whose flavor differs across platforms (GNU vs bsdtar) and misparses
    // Windows drive-letter paths.
    let file = std::fs::File::open(&archive).ok()?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(&target).ok()?;

    if script.exists() {
        Some(script)
    } else {
        None
    }
}

/// Node distribution directory name for this build target (matches the naming
/// in `scripts/prepare-runtime.mjs`).
fn node_dist_dir() -> &'static str {
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
        let (program, args) = resolve_harness_command(resource_dir.as_deref(), data_dir.as_deref());
        let mut cmd = Command::new(&program);
        cmd.args(&args).stdout(Stdio::piped()).stderr(Stdio::inherit());
        // Pin DSH_HOME to the app data directory so the agent runtime and the
        // plugin manager share one profile set (and upgrades never touch it).
        if let Some(data_dir) = &data_dir {
            cmd.env("DSH_HOME", data_dir.join("dsh"));
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
                eprintln!("[supervisor] failed to spawn {program}: {err}");
                emit_error(&app, &format!("failed to spawn harness: {err}"));
                sleep_backoff(&shutdown, backoff);
                backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
                continue;
            }
        };
        *child_pid.lock().unwrap() = Some(child.id());

        let ready = wait_until_ready(&mut child);

        match ready {
            Ok(Some(origin)) => {
                *url.lock().unwrap() = Some(origin.clone());
                emit_ready(&app, &origin);
                navigate_main_window(&app, &origin);
                backoff = INITIAL_BACKOFF_SECS;

                let status = child.wait();
                *url.lock().unwrap() = None;
                eprintln!("[supervisor] harness exited: {status:?}");
                emit_stopped(&app);
            }
            Ok(None) => {
                // Exited before reporting ready; reap and retry.
                kill_process_tree(child.id());
                let _ = child.wait();
                eprintln!("[supervisor] harness exited before ready");
            }
            Err(err) => {
                kill_process_tree(child.id());
                let _ = child.wait();
                eprintln!("[supervisor] error reading harness output: {err}");
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
                        println!("[dsh] {line}");
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
    println!("[supervisor] harness ready at {url}");
    let _ = app.emit("harness-ready", url.to_string());
}

/// Points the main window at the served loopback origin once the harness is
/// ready. Until then the window shows the local connecting page.
fn navigate_main_window(app: &AppHandle, url: &str) {
    if let Some(window) = app.get_webview_window("main") {
        match tauri::Url::parse(url) {
            Ok(parsed) => {
                if let Err(err) = window.navigate(parsed) {
                    eprintln!("[supervisor] failed to navigate main window: {err}");
                }
            }
            Err(err) => eprintln!("[supervisor] invalid harness URL {url}: {err}"),
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
        let _ = Command::new("taskkill")
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
    fn resolve_harness_command_prefers_packaged_layout() {
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
        let node_bin = runtime
            .join("node")
            .join(if cfg!(windows) { "node.exe" } else { "node" });
        if !node_bin.exists() {
            // Runtime not prepared in this environment; nothing to assert.
            return;
        }
        let (program, args) = resolve_harness_command(
            Some(&workspace),
            Some(&std::env::temp_dir().join("dsh-desktop-test-runtime")),
        );
        assert_eq!(program, node_bin.to_string_lossy());
        assert!(args[0].ends_with("dsh-web.mjs"));
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
        assert!(url.starts_with("http://127.0.0.1:"), "unexpected origin {url}");

        // Kill the whole tree so the nested dsh bin.js does not orphan.
        kill_process_tree(child.id());
        let _ = child.wait();
    }
}

