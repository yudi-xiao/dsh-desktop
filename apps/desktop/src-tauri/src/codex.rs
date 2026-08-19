//! Local Codex CLI installation, app-server lifecycle, and managed ChatGPT auth.
//!
//! Codex is deliberately installed below the application data directory when
//! the machine does not already have the pinned release. This avoids elevated
//! global npm writes and gives the desktop app a deterministic app-server
//! protocol version.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use crate::codex_usage::{publish_snapshot, CodexUsageSnapshot};
use crate::supervisor::{node_dist_dir, HarnessSupervisor};
use crate::{logging, platform::configure_background_command};

/// Latest non-alpha npm releases verified on 2026-08-17. Neither package
/// publishes an LTS dist-tag, so the desktop pins the latest release exactly.
pub const CODEX_CLI_VERSION: &str = "0.147.0";
pub const DSH_VERSION: &str = "0.1.0-rc.6";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_SESSION_ID_LEN: usize = 256;
const MAX_PROMPT_LEN: usize = 200_000;
const MAX_CONTEXT_LEN: usize = 400_000;
const MAX_SESSION_EVENTS: usize = 1_000;
const MAX_SESSION_MESSAGES: usize = 200;
const MAX_MODEL_ID_LEN: usize = 128;
const MAX_ATTACHMENTS: usize = 10;
const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;
const MAX_ATTACHMENTS_BYTES: usize = 50 * 1024 * 1024;
const MAX_GOAL_OBJECTIVE_LEN: usize = 4_000;

fn default_collaboration_mode() -> String {
    "default".into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexReasoningEffort {
    pub reasoning_effort: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexModelOption {
    #[serde(default)]
    pub id: String,
    pub model: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub default_reasoning_effort: Option<String>,
    #[serde(default)]
    pub supported_reasoning_efforts: Vec<CodexReasoningEffort>,
    #[serde(default)]
    pub input_modalities: Vec<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexModelCatalog {
    pub data: Vec<CodexModelOption>,
    pub selected_model: String,
    pub selected_effort: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCollaborationModeOption {
    pub name: String,
    pub mode: String,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCollaborationModeCatalog {
    pub data: Vec<CodexCollaborationModeOption>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexThreadGoal {
    pub thread_id: String,
    pub objective: String,
    pub status: String,
    pub token_budget: Option<u64>,
    pub tokens_used: u64,
    pub time_used_seconds: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexSessionLink {
    thread_id: String,
    cwd: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    messages: Vec<CodexSessionMessage>,
    #[serde(default = "default_collaboration_mode")]
    collaboration_mode: String,
    #[serde(default)]
    goal: Option<CodexThreadGoal>,
    #[serde(skip)]
    loaded: bool,
    #[serde(skip)]
    active_turn_id: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexSessionStore {
    #[serde(default)]
    sessions: HashMap<String, CodexSessionLink>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionEvent {
    pub seq: u64,
    pub method: String,
    pub params: Value,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionSnapshot {
    pub linked: bool,
    pub thread_id: Option<String>,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub active_turn_id: Option<String>,
    pub messages: Vec<CodexSessionMessage>,
    pub collaboration_mode: String,
    pub goal: Option<CodexThreadGoal>,
    pub latest_seq: u64,
    pub events: Vec<CodexSessionEvent>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionIndexEntry {
    pub session_id: String,
    pub thread_id: String,
    pub cwd: String,
    pub title: Option<String>,
    pub collaboration_mode: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionSendRequest {
    pub session_id: String,
    pub cwd: String,
    pub prompt: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub collaboration_mode: Option<String>,
    #[serde(default)]
    pub attachments: Vec<CodexSessionAttachment>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionGoalUpdateRequest {
    pub session_id: String,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub token_budget: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionAttachment {
    pub name: String,
    #[serde(default)]
    pub mime_type: String,
    pub data_base64: String,
}

struct StoredAttachment {
    name: String,
    path: PathBuf,
    is_image: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionSendResult {
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus {
    pub backend: String,
    pub phase: String,
    pub installed: bool,
    pub exact_version: bool,
    pub version: Option<String>,
    pub required_version: String,
    pub dsh_version: String,
    pub executable: Option<String>,
    pub managed_install: bool,
    pub app_server_running: bool,
    pub auth_mode: Option<String>,
    pub plan_type: Option<String>,
    pub account_email: Option<String>,
    pub auth_url: Option<String>,
    pub error: Option<String>,
}

impl Default for CodexStatus {
    fn default() -> Self {
        Self {
            backend: "dsh".into(),
            phase: "idle".into(),
            installed: false,
            exact_version: false,
            version: None,
            required_version: CODEX_CLI_VERSION.into(),
            dsh_version: DSH_VERSION.into(),
            executable: None,
            managed_install: false,
            app_server_running: false,
            auth_mode: None,
            plan_type: None,
            account_email: None,
            auth_url: None,
            error: None,
        }
    }
}

#[derive(Clone, Debug)]
struct CodexBinary {
    program: String,
    version: String,
    managed: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackendConfig {
    backend: Option<String>,
    codex_executable: Option<String>,
    codex_version: Option<String>,
    selected_model: Option<String>,
    selected_effort: Option<String>,
}

struct RuntimeState {
    status: CodexStatus,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    next_id: u64,
    pending: HashMap<u64, Sender<Result<Value, String>>>,
    usage: CodexUsageSnapshot,
    session_links: HashMap<String, CodexSessionLink>,
    session_events: HashMap<String, VecDeque<CodexSessionEvent>>,
    session_event_seq: u64,
    assistant_buffers: HashMap<String, String>,
    server_requests: HashMap<String, PendingServerRequest>,
    busy: bool,
}

#[derive(Clone, Debug)]
struct PendingServerRequest {
    id: Value,
    session_id: String,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            status: CodexStatus::default(),
            child: None,
            stdin: None,
            next_id: 1,
            pending: HashMap::new(),
            usage: CodexUsageSnapshot::default(),
            session_links: HashMap::new(),
            session_events: HashMap::new(),
            session_event_seq: 0,
            assistant_buffers: HashMap::new(),
            server_requests: HashMap::new(),
            busy: false,
        }
    }
}

/// Shared manager registered as Tauri state. Clones share the child process,
/// request map, and user-visible status.
#[derive(Clone, Default)]
pub struct CodexManager {
    inner: Arc<Mutex<RuntimeState>>,
}

impl CodexManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&self, app: &AppHandle) {
        let config = read_config(app);
        let session_store = read_session_store(app);
        let mut inner = self.inner.lock().unwrap();
        if let Some(backend) = config.backend {
            inner.status.backend = backend;
        }
        if let (Some(program), Some(version)) = (config.codex_executable, config.codex_version) {
            inner.status.executable = Some(program);
            inner.status.version = Some(version.clone());
            inner.status.installed = true;
            inner.status.exact_version = version == CODEX_CLI_VERSION;
        }
        inner.session_links = session_store.sessions;
        drop(inner);
        self.publish_usage(app);
    }

    pub fn bootstrap_if_selected(&self, app: AppHandle) {
        if self.status().backend != "codex" {
            return;
        }
        self.configure_async(app);
    }

    pub fn status(&self) -> CodexStatus {
        self.inner.lock().unwrap().status.clone()
    }

    pub fn refresh_detection(&self, app: &AppHandle) -> CodexStatus {
        let found = detect_codex(app);
        let mut inner = self.inner.lock().unwrap();
        apply_binary_status(&mut inner.status, found.as_ref());
        inner.status.clone()
    }

    pub fn configure_async(&self, app: AppHandle) {
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.busy {
                return;
            }
            inner.busy = true;
            inner.status.backend = "codex".into();
            inner.status.phase = "detecting".into();
            inner.status.error = None;
            inner.usage.set_loading();
        }
        emit_status(&app, &self.status());
        self.publish_usage(&app);

        let manager = self.clone();
        thread::spawn(move || {
            if let Err(error) = manager.configure(&app) {
                manager.set_error(&app, error);
            }
            manager.inner.lock().unwrap().busy = false;
        });
    }

    fn configure(&self, app: &AppHandle) -> Result<(), String> {
        write_backend_config(app, "codex", None)?;
        let binary = match detect_codex(app) {
            Some(binary) if binary.version == CODEX_CLI_VERSION => binary,
            _ => {
                self.set_phase(app, "installing");
                install_managed_codex(app)?
            }
        };

        {
            let mut inner = self.inner.lock().unwrap();
            apply_binary_status(&mut inner.status, Some(&binary));
            inner.status.phase = "startingAppServer".into();
        }
        write_backend_config(app, "codex", Some(&binary))?;
        emit_status(app, &self.status());

        self.start_app_server(app, &binary)?;
        self.set_phase(app, "checkingAccount");

        let account = self.request("account/read", json!({ "refreshToken": false }))?;
        let account_type = account
            .pointer("/account/type")
            .and_then(Value::as_str)
            .map(str::to_string);
        let plan_type = account
            .pointer("/account/planType")
            .and_then(Value::as_str)
            .map(str::to_string);
        let account_email = account
            .pointer("/account/email")
            .and_then(Value::as_str)
            .map(str::to_string);
        {
            let mut inner = self.inner.lock().unwrap();
            inner.status.auth_mode = account_type.clone();
            inner.status.plan_type = plan_type;
            inner.status.account_email = account_email;
            inner.usage.set_account(&account);
        }
        self.publish_usage(app);

        // ChatGPT managed OAuth is the preferred path even if a legacy API-key
        // login is present. Codex owns token persistence and refresh.
        if account_type.as_deref() == Some("chatgpt") {
            self.refresh_usage_best_effort(app);
            self.set_phase(app, "ready");
        } else {
            self.begin_chatgpt_login(app)?;
        }
        Ok(())
    }

    pub fn login_async(&self, app: AppHandle) {
        let manager = self.clone();
        thread::spawn(move || {
            let result = if manager.status().app_server_running {
                manager.begin_chatgpt_login(&app)
            } else {
                manager.configure(&app)
            };
            if let Err(error) = result {
                manager.set_error(&app, error);
            }
        });
    }

    pub fn logout_async(&self, app: AppHandle) {
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.busy || !inner.status.app_server_running {
                return;
            }
            inner.busy = true;
            inner.status.phase = "signingOut".into();
            inner.status.error = None;
        }
        emit_status(&app, &self.status());

        let manager = self.clone();
        thread::spawn(move || {
            match manager.request("account/logout", Value::Null) {
                Ok(_) => {
                    let mut inner = manager.inner.lock().unwrap();
                    inner.status.phase = "signedOut".into();
                    inner.status.auth_mode = None;
                    inner.status.plan_type = None;
                    inner.status.account_email = None;
                    inner.status.auth_url = None;
                    inner.status.error = None;
                    inner.usage.state = "unauthenticated".into();
                    inner.usage.error = None;
                    drop(inner);
                    emit_status(&app, &manager.status());
                    manager.publish_usage(&app);
                }
                Err(error) => manager.set_error(&app, error),
            }
            manager.inner.lock().unwrap().busy = false;
        });
    }

    pub fn usage(&self) -> CodexUsageSnapshot {
        self.inner.lock().unwrap().usage.clone()
    }

    pub fn refresh_usage_async(&self, app: AppHandle) {
        let manager = self.clone();
        thread::spawn(move || manager.refresh_usage_best_effort(&app));
    }

    pub fn model_catalog(&self, app: &AppHandle) -> Result<CodexModelCatalog, String> {
        let status = self.status();
        if status.backend != "codex" || !status.app_server_running {
            return Err("Codex app-server is not running".into());
        }
        let result = self.request(
            "model/list",
            json!({ "limit": 100, "includeHidden": false }),
        )?;
        let mut data: Vec<CodexModelOption> = serde_json::from_value(
            result
                .get("data")
                .cloned()
                .ok_or("Codex app-server did not return a model list")?,
        )
        .map_err(|error| format!("invalid Codex model list: {error}"))?;
        data.retain(|entry| !entry.hidden && !entry.model.trim().is_empty());
        if data.is_empty() {
            return Err("Codex app-server returned no selectable models".into());
        }

        let config = read_config(app);
        let selected = config
            .selected_model
            .as_deref()
            .and_then(|value| data.iter().find(|entry| entry.model == value))
            .or_else(|| data.iter().find(|entry| entry.is_default))
            .unwrap_or(&data[0]);
        let selected_model = selected.model.clone();
        let selected_effort = config
            .selected_effort
            .filter(|value| {
                selected
                    .supported_reasoning_efforts
                    .iter()
                    .any(|entry| entry.reasoning_effort == *value)
            })
            .or_else(|| {
                selected.default_reasoning_effort.clone().filter(|value| {
                    selected
                        .supported_reasoning_efforts
                        .iter()
                        .any(|entry| entry.reasoning_effort == *value)
                })
            })
            .or_else(|| {
                selected
                    .supported_reasoning_efforts
                    .first()
                    .map(|entry| entry.reasoning_effort.clone())
            });
        Ok(CodexModelCatalog {
            data,
            selected_model,
            selected_effort,
        })
    }

    pub fn collaboration_mode_catalog(&self) -> Result<CodexCollaborationModeCatalog, String> {
        let status = self.status();
        if status.backend != "codex" || !status.app_server_running {
            return Err("Codex app-server is not running".into());
        }
        let result = self.request("collaborationMode/list", json!({}))?;
        let data = result
            .get("data")
            .and_then(Value::as_array)
            .ok_or("Codex app-server did not return collaboration modes")?
            .iter()
            .filter_map(|entry| {
                let mode = entry.get("mode").and_then(Value::as_str)?;
                if !matches!(mode, "default" | "plan") {
                    return None;
                }
                Some(CodexCollaborationModeOption {
                    name: entry
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or(mode)
                        .to_string(),
                    mode: mode.to_string(),
                    model: entry
                        .get("model")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    reasoning_effort: entry
                        .get("reasoning_effort")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
            })
            .collect::<Vec<_>>();
        if !["default", "plan"]
            .iter()
            .all(|mode| data.iter().any(|entry| entry.mode == *mode))
        {
            return Err("Codex app-server did not advertise default and plan modes".into());
        }
        Ok(CodexCollaborationModeCatalog { data })
    }

    fn resolve_model_selection(
        &self,
        app: &AppHandle,
        requested_model: Option<&str>,
        requested_effort: Option<&str>,
    ) -> Result<(String, Option<String>), String> {
        if let Some(model) = requested_model {
            validate_short_identifier("model", model)?;
        }
        if let Some(effort) = requested_effort {
            validate_short_identifier("reasoning effort", effort)?;
        }
        let catalog = self.model_catalog(app)?;
        let model = requested_model.unwrap_or(&catalog.selected_model);
        let selected = catalog
            .data
            .iter()
            .find(|entry| entry.model == model)
            .ok_or_else(|| format!("Codex model is not available: {model}"))?;
        let effort = requested_effort.map(str::to_string).or_else(|| {
            if model == catalog.selected_model {
                catalog.selected_effort.clone()
            } else {
                selected.default_reasoning_effort.clone()
            }
        });
        if let Some(value) = effort.as_deref() {
            if !selected
                .supported_reasoning_efforts
                .iter()
                .any(|entry| entry.reasoning_effort == value)
            {
                return Err(format!(
                    "Reasoning effort '{value}' is not supported by model '{model}'"
                ));
            }
        }
        write_model_config(app, model, effort.as_deref())?;
        Ok((model.to_string(), effort))
    }

    pub fn session_snapshot(
        &self,
        session_id: &str,
        after_seq: u64,
    ) -> Result<CodexSessionSnapshot, String> {
        validate_session_id(session_id)?;
        let inner = self.inner.lock().unwrap();
        let link = inner.session_links.get(session_id);
        let events = inner
            .session_events
            .get(session_id)
            .map(|queue| {
                queue
                    .iter()
                    .filter(|event| event.seq > after_seq)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(CodexSessionSnapshot {
            linked: link.is_some(),
            thread_id: link.map(|value| value.thread_id.clone()),
            cwd: link.map(|value| value.cwd.clone()),
            title: link.and_then(|value| value.title.clone()),
            active_turn_id: link.and_then(|value| value.active_turn_id.clone()),
            messages: link.map(|value| value.messages.clone()).unwrap_or_default(),
            collaboration_mode: link
                .map(|value| value.collaboration_mode.clone())
                .unwrap_or_else(default_collaboration_mode),
            goal: link.and_then(|value| value.goal.clone()),
            latest_seq: inner.session_event_seq,
            events,
        })
    }

    pub fn session_index(&self) -> Vec<CodexSessionIndexEntry> {
        let inner = self.inner.lock().unwrap();
        let mut entries = inner
            .session_links
            .iter()
            .map(|(session_id, link)| CodexSessionIndexEntry {
                session_id: session_id.clone(),
                thread_id: link.thread_id.clone(),
                cwd: link.cwd.clone(),
                title: link.title.clone(),
                collaboration_mode: link.collaboration_mode.clone(),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        entries
    }

    pub fn session_send(
        &self,
        app: &AppHandle,
        request: CodexSessionSendRequest,
    ) -> Result<CodexSessionSendResult, String> {
        validate_session_id(&request.session_id)?;
        validate_text(
            "prompt",
            &request.prompt,
            MAX_PROMPT_LEN,
            !request.attachments.is_empty(),
        )?;
        if let Some(context) = request.context.as_deref() {
            validate_text("context", context, MAX_CONTEXT_LEN, true)?;
        }
        let cwd = validate_cwd(&request.cwd)?;
        let status = self.status();
        if status.backend != "codex"
            || !status.app_server_running
            || status.auth_mode.as_deref() != Some("chatgpt")
            || status.phase != "ready"
        {
            return Err(
                "Codex is not ready. Open dsh Settings → Codex and complete ChatGPT login first."
                    .into(),
            );
        }
        let (model, effort) =
            self.resolve_model_selection(app, request.model.as_deref(), request.effort.as_deref())?;
        let collaboration_mode =
            validate_collaboration_mode(request.collaboration_mode.as_deref())?.to_string();
        let existing = self
            .inner
            .lock()
            .unwrap()
            .session_links
            .get(&request.session_id)
            .cloned();
        let is_new = existing.is_none();
        let thread_id = if let Some(link) = existing {
            if link.cwd != cwd {
                return Err(
                    "This dsh session is already linked to a Codex thread in another workspace. Start a new Codex thread first."
                        .into(),
                );
            }
            if !link.loaded {
                self.request(
                    "thread/resume",
                    json!({
                        "threadId": link.thread_id,
                        "cwd": cwd,
                        "approvalPolicy": "untrusted",
                        "sandbox": "workspace-write"
                    }),
                )?;
                if let Some(current) = self
                    .inner
                    .lock()
                    .unwrap()
                    .session_links
                    .get_mut(&request.session_id)
                {
                    current.loaded = true;
                }
            }
            link.thread_id
        } else {
            let result = self.request(
                "thread/start",
                json!({
                    "cwd": cwd,
                    "model": model,
                    "approvalPolicy": "untrusted",
                    "sandbox": "workspace-write",
                    "serviceName": "dsh_desktop"
                }),
            )?;
            let thread_id = result
                .pointer("/thread/id")
                .and_then(Value::as_str)
                .ok_or("Codex app-server did not return a thread id")?
                .to_string();
            {
                let mut inner = self.inner.lock().unwrap();
                inner.session_links.insert(
                    request.session_id.clone(),
                    CodexSessionLink {
                        thread_id: thread_id.clone(),
                        cwd: cwd.clone(),
                        title: clean_title(request.title.as_deref()),
                        messages: Vec::new(),
                        collaboration_mode: collaboration_mode.clone(),
                        goal: None,
                        loaded: true,
                        active_turn_id: None,
                    },
                );
                push_session_event_locked(
                    &mut inner,
                    &request.session_id,
                    "desktop/session-linked",
                    json!({ "threadId": thread_id, "cwd": cwd }),
                );
            }
            write_session_store(app, &self.inner.lock().unwrap().session_links)?;
            if let Some(title) = clean_title(request.title.as_deref()) {
                let _ = self.request(
                    "thread/name/set",
                    json!({ "threadId": thread_id, "name": title }),
                );
            }
            thread_id
        };

        let attachments =
            store_session_attachments(app, &request.session_id, &request.attachments)?;
        let mut prompt = if is_new {
            match request.context.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                Some(context) => format!(
                    "Continue this work from a DeepSeek Harness session. Treat the transferred transcript as context, verify the repository state yourself, and perform the user's current request.\n\n<dsh-session-context>\n{context}\n</dsh-session-context>\n\n<current-request>\n{}\n</current-request>",
                    request.prompt.trim()
                ),
                None => request.prompt.trim().to_string(),
            }
        } else {
            request.prompt.trim().to_string()
        };
        if prompt.is_empty() {
            prompt = "Review and process the attached files.".into();
        }
        if !attachments.is_empty() {
            prompt.push_str("\n\nThe user selected these local attachments in DSH Desktop. Read them from the exact paths below when needed:\n<local-attachments>");
            for attachment in &attachments {
                prompt.push_str(&format!(
                    "\n- {}: {}",
                    attachment.name,
                    attachment.path.to_string_lossy()
                ));
            }
            prompt.push_str("\n</local-attachments>");
        }
        let mut input = vec![json!({ "type": "text", "text": prompt })];
        input.extend(
            attachments
                .iter()
                .filter(|attachment| attachment.is_image)
                .map(|attachment| {
                    json!({
                        "type": "localImage",
                        "path": attachment.path.to_string_lossy()
                    })
                }),
        );
        let mut turn_params = json!({
            "threadId": thread_id,
            "input": input,
            "cwd": cwd,
            "model": model,
            "approvalPolicy": "untrusted",
            "collaborationMode": {
                "mode": collaboration_mode,
                "settings": {
                    "model": model,
                    "reasoning_effort": effort,
                    "developer_instructions": null
                }
            },
            "sandboxPolicy": {
                "type": "workspaceWrite",
                "writableRoots": [cwd],
                "networkAccess": false,
                "excludeTmpdirEnvVar": false,
                "excludeSlashTmp": false
            }
        });
        if let Some(effort) = effort {
            turn_params["effort"] = Value::String(effort);
        }
        let result = self.request("turn/start", turn_params)?;
        let turn_id = result
            .pointer("/turn/id")
            .and_then(Value::as_str)
            .ok_or("Codex app-server did not return a turn id")?
            .to_string();
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(link) = inner.session_links.get_mut(&request.session_id) {
                link.active_turn_id = Some(turn_id.clone());
                link.collaboration_mode = collaboration_mode;
                push_message(
                    &mut link.messages,
                    CodexSessionMessage {
                        id: format!("user-{turn_id}"),
                        role: "user".into(),
                        text: request.prompt.trim().to_string(),
                        created_at: unix_millis(),
                    },
                );
            }
            push_session_event_locked(
                &mut inner,
                &request.session_id,
                "desktop/user-message",
                json!({ "threadId": thread_id, "turnId": turn_id }),
            );
        }
        write_session_store(app, &self.inner.lock().unwrap().session_links)?;
        Ok(CodexSessionSendResult { thread_id, turn_id })
    }

    fn session_thread_id(&self, session_id: &str) -> Result<String, String> {
        validate_session_id(session_id)?;
        self.inner
            .lock()
            .unwrap()
            .session_links
            .get(session_id)
            .map(|link| link.thread_id.clone())
            .ok_or_else(|| "This dsh session is not linked to Codex".into())
    }

    pub fn session_goal_get(
        &self,
        app: &AppHandle,
        session_id: &str,
    ) -> Result<Option<CodexThreadGoal>, String> {
        let thread_id = self.session_thread_id(session_id)?;
        let result = self.request("thread/goal/get", json!({ "threadId": thread_id }))?;
        let goal = parse_thread_goal(result.get("goal"))?;
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(link) = inner.session_links.get_mut(session_id) {
                link.goal = goal.clone();
            }
        }
        write_session_store(app, &self.inner.lock().unwrap().session_links)?;
        Ok(goal)
    }

    pub fn session_goal_update(
        &self,
        app: &AppHandle,
        request: CodexSessionGoalUpdateRequest,
    ) -> Result<CodexThreadGoal, String> {
        let thread_id = self.session_thread_id(&request.session_id)?;
        if request.objective.is_none() && request.status.is_none() && request.token_budget.is_none()
        {
            return Err("A goal update must change the objective, status, or token budget".into());
        }
        let objective = request
            .objective
            .as_deref()
            .map(validate_goal_objective)
            .transpose()?;
        if let Some(status) = request.status.as_deref() {
            if !matches!(status, "active" | "paused") {
                return Err("Goal status must be 'active' or 'paused'".into());
            }
        }
        if request.token_budget == Some(0) {
            return Err("Goal token budget must be greater than zero".into());
        }

        let mut params = serde_json::Map::new();
        params.insert("threadId".into(), Value::String(thread_id));
        if let Some(objective) = objective {
            params.insert("objective".into(), Value::String(objective));
        }
        if let Some(status) = request.status {
            params.insert("status".into(), Value::String(status));
        }
        if let Some(token_budget) = request.token_budget {
            params.insert("tokenBudget".into(), Value::Number(token_budget.into()));
        }
        let result = self.request("thread/goal/set", Value::Object(params))?;
        let goal = parse_thread_goal(result.get("goal"))?
            .ok_or("Codex app-server did not return the updated goal")?;
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(link) = inner.session_links.get_mut(&request.session_id) {
                link.goal = Some(goal.clone());
            }
            push_session_event_locked(
                &mut inner,
                &request.session_id,
                "desktop/goal-updated",
                json!({ "goal": goal }),
            );
        }
        write_session_store(app, &self.inner.lock().unwrap().session_links)?;
        Ok(goal)
    }

    pub fn session_goal_clear(&self, app: &AppHandle, session_id: &str) -> Result<(), String> {
        let thread_id = self.session_thread_id(session_id)?;
        self.request("thread/goal/clear", json!({ "threadId": thread_id }))?;
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(link) = inner.session_links.get_mut(session_id) {
                link.goal = None;
            }
            push_session_event_locked(
                &mut inner,
                session_id,
                "desktop/goal-cleared",
                json!({ "threadId": thread_id }),
            );
        }
        write_session_store(app, &self.inner.lock().unwrap().session_links)
    }

    pub fn session_interrupt(&self, session_id: &str) -> Result<(), String> {
        validate_session_id(session_id)?;
        let (thread_id, turn_id) = {
            let inner = self.inner.lock().unwrap();
            let link = inner
                .session_links
                .get(session_id)
                .ok_or("This dsh session is not linked to Codex")?;
            (
                link.thread_id.clone(),
                link.active_turn_id
                    .clone()
                    .ok_or("There is no active Codex turn")?,
            )
        };
        self.request(
            "turn/interrupt",
            json!({ "threadId": thread_id, "turnId": turn_id }),
        )?;
        Ok(())
    }

    pub fn session_approve(
        &self,
        session_id: &str,
        request_id: &Value,
        decision: &str,
    ) -> Result<(), String> {
        validate_session_id(session_id)?;
        if !matches!(
            decision,
            "accept" | "acceptForSession" | "decline" | "cancel"
        ) {
            return Err("Unsupported Codex approval decision".into());
        }
        let mut inner = self.inner.lock().unwrap();
        let key = request_id_key(request_id)?;
        let pending = inner
            .server_requests
            .get(&key)
            .ok_or("Codex approval request is no longer pending")?;
        if pending.session_id != session_id {
            return Err("Codex approval request belongs to another session".into());
        }
        let response_id = pending.id.clone();
        let stdin = inner
            .stdin
            .as_mut()
            .ok_or("Codex app-server is not running")?;
        let response = json!({ "id": response_id, "result": { "decision": decision } });
        writeln!(stdin, "{response}").map_err(|error| error.to_string())?;
        stdin.flush().map_err(|error| error.to_string())?;
        inner.server_requests.remove(&key);
        Ok(())
    }

    pub fn session_reset(&self, app: &AppHandle, session_id: &str) -> Result<(), String> {
        validate_session_id(session_id)?;
        let mut inner = self.inner.lock().unwrap();
        if inner
            .session_links
            .get(session_id)
            .and_then(|link| link.active_turn_id.as_ref())
            .is_some()
        {
            return Err("Stop the active Codex turn before starting a new thread".into());
        }
        inner.session_links.remove(session_id);
        inner.session_events.remove(session_id);
        inner
            .server_requests
            .retain(|_, pending| pending.session_id != session_id);
        let links = inner.session_links.clone();
        drop(inner);
        write_session_store(app, &links)
    }

    fn refresh_usage_best_effort(&self, app: &AppHandle) {
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.status.auth_mode.as_deref() != Some("chatgpt") {
                inner.usage.state = "unauthenticated".into();
                inner.usage.error = None;
                drop(inner);
                self.publish_usage(app);
                return;
            }
            inner.usage.set_loading();
        }
        self.publish_usage(app);

        // account/updated deliberately carries no email address. Re-read the
        // account after managed OAuth so the UI receives the complete identity
        // without ever handling the underlying tokens.
        if let Ok(account) = self.request("account/read", json!({ "refreshToken": false })) {
            let auth_mode = account
                .pointer("/account/type")
                .and_then(Value::as_str)
                .map(str::to_string);
            let plan_type = account
                .pointer("/account/planType")
                .and_then(Value::as_str)
                .map(str::to_string);
            let account_email = account
                .pointer("/account/email")
                .and_then(Value::as_str)
                .map(str::to_string);
            let mut inner = self.inner.lock().unwrap();
            inner.status.auth_mode = auth_mode;
            inner.status.plan_type = plan_type;
            inner.status.account_email = account_email;
            inner.usage.set_account(&account);
        }

        match self.request("account/rateLimits/read", json!({})) {
            Ok(rate_limits) => {
                self.inner
                    .lock()
                    .unwrap()
                    .usage
                    .apply_rate_limits(&rate_limits);
                // This method is documented by current app-server releases but
                // is not present in every pinned protocol schema. Treat it as an
                // optional enhancement and keep the rate-limit view usable when
                // an older server returns method-not-found.
                if let Ok(account_usage) = self.request("account/usage/read", json!({})) {
                    self.inner
                        .lock()
                        .unwrap()
                        .usage
                        .apply_account_usage(&account_usage);
                }
            }
            Err(error) => self
                .inner
                .lock()
                .unwrap()
                .usage
                .set_unavailable(Some(error)),
        }
        self.publish_usage(app);
    }

    fn publish_usage(&self, app: &AppHandle) {
        let snapshot = {
            let mut inner = self.inner.lock().unwrap();
            let status = inner.status.clone();
            inner.usage.set_runtime(
                status.backend,
                status.phase,
                status.version,
                status.required_version,
                status.app_server_running,
                status.auth_mode,
                status.managed_install,
            );
            inner.usage.clone()
        };
        if let Err(error) = publish_snapshot(app, &snapshot) {
            logging::error(
                "codex",
                format!("failed to publish usage snapshot: {error}"),
            );
        }
    }

    fn begin_chatgpt_login(&self, app: &AppHandle) -> Result<(), String> {
        self.set_phase(app, "startingLogin");
        let result = self.request(
            "account/login/start",
            json!({
                "type": "chatgpt",
                "useHostedLoginSuccessPage": true,
                "appBrand": "chatgpt"
            }),
        )?;
        let auth_url = result
            .get("authUrl")
            .and_then(Value::as_str)
            .ok_or("Codex app-server did not return an OAuth URL")?
            .to_string();
        {
            let mut inner = self.inner.lock().unwrap();
            inner.status.phase = "waitingForLogin".into();
            inner.status.auth_url = Some(auth_url.clone());
        }
        emit_status(app, &self.status());
        tauri_plugin_opener::open_url(&auth_url, None::<&str>)
            .map_err(|error| format!("failed to open ChatGPT login: {error}"))?;
        Ok(())
    }

    fn start_app_server(&self, app: &AppHandle, binary: &CodexBinary) -> Result<(), String> {
        if self.status().app_server_running {
            return Ok(());
        }

        let mut command = Command::new(&binary.program);
        configure_background_command(&mut command);
        command
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start Codex app-server: {error}"))?;
        logging::info(
            "codex",
            format!("started app-server with CLI {}", binary.version),
        );
        let stdin = child
            .stdin
            .take()
            .ok_or("Codex app-server stdin unavailable")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Codex app-server stdout unavailable")?;
        let stderr = child.stderr.take();

        {
            let mut inner = self.inner.lock().unwrap();
            inner.child = Some(child);
            inner.stdin = Some(stdin);
            inner.status.app_server_running = true;
            inner.status.error = None;
        }

        let reader_manager = self.clone();
        let reader_app = app.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => reader_manager.handle_line(&reader_app, &line),
                    Err(error) => {
                        logging::error("codex", format!("app-server stdout error: {error}"));
                        break;
                    }
                }
            }
            reader_manager.mark_stopped(&reader_app, "Codex app-server stopped");
        });
        if let Some(stderr) = stderr {
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    logging::warn("codex", line);
                }
            });
        }

        self.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "dsh_desktop",
                    "title": "DSH Desktop",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": { "experimentalApi": true }
            }),
        )?;
        self.notify("initialized", json!({}))?;
        emit_status(app, &self.status());
        Ok(())
    }

    pub fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let (tx, rx) = mpsc::channel();
        let id;
        {
            let mut inner = self.inner.lock().unwrap();
            id = inner.next_id;
            inner.next_id += 1;
            inner.pending.insert(id, tx);
            let message = if params.is_null() {
                json!({ "method": method, "id": id })
            } else {
                json!({ "method": method, "id": id, "params": params })
            };
            let write_result = match inner.stdin.as_mut() {
                Some(stdin) => writeln!(stdin, "{message}")
                    .and_then(|_| stdin.flush())
                    .map_err(|error| error.to_string()),
                None => Err("Codex app-server is not running".into()),
            };
            if let Err(error) = write_result {
                inner.pending.remove(&id);
                return Err(error);
            }
        }
        match rx.recv_timeout(REQUEST_TIMEOUT) {
            Ok(result) => result,
            Err(_) => {
                self.inner.lock().unwrap().pending.remove(&id);
                Err(format!("Codex app-server request timed out: {method}"))
            }
        }
    }

    fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let stdin = inner
            .stdin
            .as_mut()
            .ok_or("Codex app-server is not running")?;
        let message = json!({ "method": method, "params": params });
        writeln!(stdin, "{message}").map_err(|error| error.to_string())?;
        stdin.flush().map_err(|error| error.to_string())
    }

    fn handle_line(&self, app: &AppHandle, line: &str) {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(error) => {
                logging::error("codex", format!("invalid app-server JSON: {error}"));
                return;
            }
        };

        if let Some(id_value) = message.get("id") {
            if let Some(id) = id_value.as_u64() {
                let sender = self.inner.lock().unwrap().pending.remove(&id);
                if let Some(sender) = sender {
                    let result = if let Some(error) = message.get("error") {
                        Err(error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("Codex app-server request failed")
                            .to_string())
                    } else {
                        Ok(message.get("result").cloned().unwrap_or(Value::Null))
                    };
                    let _ = sender.send(result);
                    return;
                }
            }
            if message.get("method").and_then(Value::as_str).is_some() {
                self.handle_server_request(app, id_value.clone(), &message);
            }
            return;
        }

        if let Some(method) = message.get("method").and_then(Value::as_str) {
            let mut refresh_usage = false;
            match method {
                "account/updated" => {
                    let params = message.get("params").unwrap_or(&Value::Null);
                    let mut inner = self.inner.lock().unwrap();
                    inner.status.auth_mode = params
                        .get("authMode")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let plan_type = params
                        .get("planType")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    inner.status.plan_type = plan_type.clone();
                    inner.usage.set_plan_type(plan_type);
                    if inner.status.auth_mode.as_deref() == Some("chatgpt") {
                        inner.status.phase = "ready".into();
                        inner.status.auth_url = None;
                        inner.status.error = None;
                        inner.usage.state = "loading".into();
                        inner.usage.error = None;
                        refresh_usage = true;
                    } else {
                        inner.usage.state = "unauthenticated".into();
                        if inner.status.auth_mode.is_none() {
                            inner.status.phase = "signedOut".into();
                            inner.status.plan_type = None;
                            inner.status.account_email = None;
                            inner.status.auth_url = None;
                            inner.status.error = None;
                        }
                    }
                }
                "account/login/completed" => {
                    let params = message.get("params").unwrap_or(&Value::Null);
                    if params.get("success").and_then(Value::as_bool) == Some(false) {
                        let error = params
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("ChatGPT login failed")
                            .to_string();
                        let mut inner = self.inner.lock().unwrap();
                        inner.status.phase = "error".into();
                        inner.status.error = Some(error);
                    }
                }
                "account/rateLimits/updated" => {
                    let params = message.get("params").unwrap_or(&Value::Null);
                    self.inner
                        .lock()
                        .unwrap()
                        .usage
                        .apply_rate_limit_notification(params);
                }
                "thread/tokenUsage/updated" => {
                    let params = message.get("params").unwrap_or(&Value::Null);
                    self.inner.lock().unwrap().usage.apply_thread_usage(params);
                }
                "turn/completed" => refresh_usage = true,
                _ => {}
            }
            self.handle_session_notification(app, method, message.get("params"));
            let _ = app.emit("codex-app-server-event", message.clone());
            emit_status(app, &self.status());
            self.publish_usage(app);
            if refresh_usage {
                self.refresh_usage_async(app.clone());
            }
        }
    }

    fn handle_server_request(&self, app: &AppHandle, id: Value, message: &Value) {
        let method = match message.get("method").and_then(Value::as_str) {
            Some(method) => method,
            None => return,
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let thread_id = extract_thread_id(&params);
        let session_id = thread_id.as_deref().and_then(|thread_id| {
            let inner = self.inner.lock().unwrap();
            session_for_thread(&inner.session_links, thread_id)
        });
        let supported = matches!(
            method,
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval"
        );
        if let Some(session_id) = session_id.filter(|_| supported) {
            let safe_params = sanitize_server_request(method, &id, &params);
            let mut inner = self.inner.lock().unwrap();
            let key = match request_id_key(&id) {
                Ok(key) => key,
                Err(error) => {
                    logging::warn("codex", error);
                    return;
                }
            };
            inner.server_requests.insert(
                key,
                PendingServerRequest {
                    id: id.clone(),
                    session_id: session_id.clone(),
                },
            );
            push_session_event_locked(&mut inner, &session_id, method, safe_params);
            drop(inner);
            let _ = app.emit("codex-session-updated", session_id);
            return;
        }

        // Never leave Codex blocked on a request the dsh adapter cannot safely
        // render. Unsupported tool/permission elicitation is denied by default.
        let mut inner = self.inner.lock().unwrap();
        if let Some(stdin) = inner.stdin.as_mut() {
            let response = json!({
                "id": id,
                "error": { "code": -32601, "message": "This request is not supported by the DSH Desktop session adapter" }
            });
            let _ = writeln!(stdin, "{response}");
            let _ = stdin.flush();
        }
        logging::warn(
            "codex",
            format!("declined unsupported app-server request: {method}"),
        );
    }

    fn handle_session_notification(&self, app: &AppHandle, method: &str, params: Option<&Value>) {
        if !is_session_notification(method) {
            return;
        }
        let params = params.cloned().unwrap_or(Value::Null);
        let Some(thread_id) = extract_thread_id(&params) else {
            return;
        };
        let session_id = {
            let inner = self.inner.lock().unwrap();
            session_for_thread(&inner.session_links, &thread_id)
        };
        let Some(session_id) = session_id else {
            return;
        };

        let mut persist = false;
        {
            let mut inner = self.inner.lock().unwrap();
            match method {
                "item/agentMessage/delta" | "item/plan/delta" => {
                    if let (Some(item_id), Some(delta)) = (
                        params.get("itemId").and_then(Value::as_str),
                        params.get("delta").and_then(Value::as_str),
                    ) {
                        let buffer = inner
                            .assistant_buffers
                            .entry(item_id.to_string())
                            .or_default();
                        if buffer.len() < MAX_PROMPT_LEN {
                            let remaining = MAX_PROMPT_LEN - buffer.len();
                            buffer.push_str(&delta.chars().take(remaining).collect::<String>());
                        }
                    }
                }
                "item/completed" => {
                    let item = params.get("item").unwrap_or(&Value::Null);
                    let item_type = item.get("type").and_then(Value::as_str);
                    if matches!(item_type, Some("agentMessage" | "plan")) {
                        let item_id = item
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("assistant");
                        let buffered = inner.assistant_buffers.remove(item_id);
                        let text = item
                            .get("text")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .or(buffered)
                            .unwrap_or_default();
                        if !text.trim().is_empty() {
                            if let Some(link) = inner.session_links.get_mut(&session_id) {
                                push_message(
                                    &mut link.messages,
                                    CodexSessionMessage {
                                        id: item_id.to_string(),
                                        role: if item_type == Some("plan") {
                                            "plan".into()
                                        } else {
                                            "assistant".into()
                                        },
                                        text,
                                        created_at: unix_millis(),
                                    },
                                );
                                persist = true;
                            }
                        }
                    }
                }
                "turn/started" => {
                    if let Some(turn_id) = params
                        .pointer("/turn/id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                    {
                        if let Some(link) = inner.session_links.get_mut(&session_id) {
                            link.active_turn_id = Some(turn_id);
                        }
                    }
                }
                "turn/completed" => {
                    if let Some(link) = inner.session_links.get_mut(&session_id) {
                        link.active_turn_id = None;
                        persist = true;
                    }
                }
                "thread/goal/updated" => {
                    if let Ok(goal) = parse_thread_goal(params.get("goal")) {
                        if let Some(link) = inner.session_links.get_mut(&session_id) {
                            link.goal = goal;
                            persist = true;
                        }
                    }
                }
                "thread/goal/cleared" => {
                    if let Some(link) = inner.session_links.get_mut(&session_id) {
                        link.goal = None;
                        persist = true;
                    }
                }
                "serverRequest/resolved" => {
                    if let Some(request_id) = params.get("requestId") {
                        if let Ok(key) = request_id_key(request_id) {
                            inner.server_requests.remove(&key);
                        }
                    }
                }
                _ => {}
            }
            push_session_event_locked(
                &mut inner,
                &session_id,
                method,
                sanitize_session_params(method, &params),
            );
        }
        if persist {
            if let Err(error) = write_session_store(app, &self.inner.lock().unwrap().session_links)
            {
                logging::error(
                    "codex",
                    format!("failed to persist session adapter: {error}"),
                );
            }
        }
        let _ = app.emit("codex-session-updated", session_id);
    }

    fn set_phase(&self, app: &AppHandle, phase: &str) {
        self.inner.lock().unwrap().status.phase = phase.into();
        emit_status(app, &self.status());
        self.publish_usage(app);
    }

    fn set_error(&self, app: &AppHandle, error: String) {
        logging::error("codex", &error);
        let mut inner = self.inner.lock().unwrap();
        inner.status.phase = "error".into();
        inner.status.error = Some(error.clone());
        inner.usage.set_unavailable(Some(error));
        drop(inner);
        emit_status(app, &self.status());
        self.publish_usage(app);
    }

    fn mark_stopped(&self, app: &AppHandle, message: &str) {
        logging::warn("codex", message);
        let mut inner = self.inner.lock().unwrap();
        inner.child = None;
        inner.stdin = None;
        inner.server_requests.clear();
        inner.assistant_buffers.clear();
        for link in inner.session_links.values_mut() {
            link.loaded = false;
            link.active_turn_id = None;
        }
        inner.status.app_server_running = false;
        inner.server_requests.clear();
        inner.assistant_buffers.clear();
        for link in inner.session_links.values_mut() {
            link.loaded = false;
            link.active_turn_id = None;
        }
        if inner.status.backend == "codex" && inner.status.phase != "error" {
            inner.status.phase = "error".into();
            inner.status.error = Some(message.into());
        }
        for (_, pending) in inner.pending.drain() {
            let _ = pending.send(Err(message.into()));
        }
        inner.usage.set_unavailable(Some(message.into()));
        drop(inner);
        emit_status(app, &self.status());
        self.publish_usage(app);
    }

    pub fn select_dsh(&self, app: &AppHandle) -> Result<(), String> {
        self.shutdown();
        {
            let mut inner = self.inner.lock().unwrap();
            inner.status.backend = "dsh".into();
            inner.status.phase = "idle".into();
            inner.status.error = None;
            inner.status.auth_url = None;
            inner.usage.set_unavailable(None);
        }
        write_backend_config(app, "dsh", None)?;
        emit_status(app, &self.status());
        self.publish_usage(app);
        Ok(())
    }

    pub fn shutdown(&self) {
        logging::info("codex", "stopping app-server");
        let mut inner = self.inner.lock().unwrap();
        inner.stdin = None;
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        inner.status.app_server_running = false;
        for (_, pending) in inner.pending.drain() {
            let _ = pending.send(Err("Codex app-server stopped".into()));
        }
    }
}

fn emit_status(app: &AppHandle, status: &CodexStatus) {
    let _ = app.emit("codex-status", status.clone());
}

fn apply_binary_status(status: &mut CodexStatus, binary: Option<&CodexBinary>) {
    status.installed = binary.is_some();
    status.exact_version = binary
        .map(|binary| binary.version == CODEX_CLI_VERSION)
        .unwrap_or(false);
    status.version = binary.map(|binary| binary.version.clone());
    status.executable = binary.map(|binary| binary.program.clone());
    status.managed_install = binary.map(|binary| binary.managed).unwrap_or(false);
}

fn detect_codex(app: &AppHandle) -> Option<CodexBinary> {
    let config = read_config(app);
    let managed = managed_codex_executable(app);
    let mut candidates: Vec<(String, bool)> = Vec::new();
    if let Ok(program) = std::env::var("DSH_DESKTOP_CODEX_CMD") {
        candidates.push((program, false));
    }
    if let Some(program) = config.codex_executable {
        candidates.push((program.clone(), Path::new(&program) == managed));
    }
    candidates.push((managed.to_string_lossy().into_owned(), true));
    if cfg!(windows) {
        candidates.push(("codex.cmd".into(), false));
        candidates.push(("codex.exe".into(), false));
    }
    candidates.push(("codex".into(), false));

    let mut fallback = None;
    for (program, managed) in candidates {
        if let Some(version) = codex_version(&program) {
            let binary = CodexBinary {
                program,
                version,
                managed,
            };
            if binary.version == CODEX_CLI_VERSION {
                return Some(binary);
            }
            if fallback.is_none() {
                fallback = Some(binary);
            }
        }
    }
    fallback
}

fn codex_version(program: &str) -> Option<String> {
    let mut command = Command::new(program);
    configure_background_command(&mut command);
    let output = command.arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_codex_version(&stdout)
}

fn parse_codex_version(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .find(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        .map(|part| part.trim().to_string())
}

fn managed_prefix(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_default()
        .join("codex-cli")
        .join(CODEX_CLI_VERSION)
}

fn managed_codex_executable(app: &AppHandle) -> PathBuf {
    let prefix = managed_prefix(app);
    if cfg!(windows) {
        prefix.join("codex.cmd")
    } else {
        prefix.join("bin").join("codex")
    }
}

fn install_managed_codex(app: &AppHandle) -> Result<CodexBinary, String> {
    let prefix = managed_prefix(app);
    fs::create_dir_all(&prefix).map_err(|error| error.to_string())?;
    let (program, initial_args) = resolve_npm_command(app);
    let mut command = Command::new(&program);
    configure_background_command(&mut command);
    command.args(initial_args);
    command
        .args(["install", "--global", "--prefix"])
        .arg(&prefix)
        .arg(format!("@openai/codex@{CODEX_CLI_VERSION}"))
        .args(["--no-audit", "--no-fund"]);
    let output = command
        .output()
        .map_err(|error| format!("failed to run npm: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Codex CLI installation failed:\n{stdout}\n{stderr}"
        ));
    }
    let executable = managed_codex_executable(app);
    let program = executable.to_string_lossy().into_owned();
    let version = codex_version(&program).ok_or("installed Codex CLI did not start")?;
    if version != CODEX_CLI_VERSION {
        return Err(format!(
            "installed Codex CLI version {version}, expected {CODEX_CLI_VERSION}"
        ));
    }
    Ok(CodexBinary {
        program,
        version,
        managed: true,
    })
}

/// Returns a program plus leading arguments for npm. Packaged builds prefer
/// the npm CLI that ships with the portable Node runtime; development falls
/// back to npm from PATH.
fn resolve_npm_command(app: &AppHandle) -> (String, Vec<String>) {
    if let Ok(program) = std::env::var("DSH_DESKTOP_NPM_CMD") {
        return (program, Vec::new());
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        let root = resource_dir
            .join("vendor")
            .join("runtime")
            .join(node_dist_dir())
            .join("node");
        let (node, npm_cli) = if cfg!(windows) {
            (
                root.join("node.exe"),
                root.join("node_modules/npm/bin/npm-cli.js"),
            )
        } else {
            (
                root.join("bin/node"),
                root.join("lib/node_modules/npm/bin/npm-cli.js"),
            )
        };
        if node.exists() && npm_cli.exists() {
            return (
                node.to_string_lossy().into_owned(),
                vec![npm_cli.to_string_lossy().into_owned()],
            );
        }
    }
    ("npm".into(), Vec::new())
}

fn config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_default()
        .join("backend.json")
}

fn read_config(app: &AppHandle) -> BackendConfig {
    fs::read_to_string(config_path(app))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_backend_config(
    app: &AppHandle,
    backend: &str,
    binary: Option<&CodexBinary>,
) -> Result<(), String> {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let old = read_config(app);
    let config = BackendConfig {
        backend: Some(backend.into()),
        codex_executable: binary
            .map(|binary| binary.program.clone())
            .or(old.codex_executable),
        codex_version: binary
            .map(|binary| binary.version.clone())
            .or(old.codex_version),
        selected_model: old.selected_model,
        selected_effort: old.selected_effort,
    };
    let text = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

fn write_model_config(app: &AppHandle, model: &str, effort: Option<&str>) -> Result<(), String> {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let old = read_config(app);
    let config = BackendConfig {
        backend: old.backend,
        codex_executable: old.codex_executable,
        codex_version: old.codex_version,
        selected_model: Some(model.into()),
        selected_effort: effort.map(str::to_string),
    };
    let text = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

fn session_store_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_default()
        .join("codex-sessions.json")
}

fn attachment_root(app: &AppHandle, session_id: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve the application data directory: {error}"))?
        .join("codex-attachments")
        .join(session_id))
}

fn safe_attachment_name(name: &str, index: usize) -> Result<String, String> {
    let source = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .trim();
    if source.is_empty() || source == "." || source == ".." {
        return Err("An attachment has an invalid file name".into());
    }
    let sanitized: String = source
        .chars()
        .take(120)
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '.' | '-' | '_' | ' ') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    Ok(format!("{index:02}-{sanitized}"))
}

fn is_codex_image(mime_type: &str, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        (mime_type, extension.as_str()),
        ("image/png", "png")
            | ("image/jpeg", "jpg" | "jpeg")
            | ("image/gif", "gif")
            | ("image/webp", "webp")
    )
}

fn store_session_attachments(
    app: &AppHandle,
    session_id: &str,
    attachments: &[CodexSessionAttachment],
) -> Result<Vec<StoredAttachment>, String> {
    if attachments.len() > MAX_ATTACHMENTS {
        return Err(format!(
            "A message can include at most {MAX_ATTACHMENTS} attachments"
        ));
    }
    if attachments.is_empty() {
        return Ok(Vec::new());
    }

    let root = attachment_root(app, session_id)?;
    fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create the attachment directory: {error}"))?;
    let stamp = unix_millis();
    let mut total_bytes = 0usize;
    let mut stored = Vec::with_capacity(attachments.len());
    for (index, attachment) in attachments.iter().enumerate() {
        if attachment.data_base64.len() > (MAX_ATTACHMENT_BYTES * 4 / 3) + 8 {
            return Err(format!(
                "Attachment '{}' is larger than 20 MB",
                attachment.name
            ));
        }
        let bytes = BASE64_STANDARD
            .decode(&attachment.data_base64)
            .map_err(|_| format!("Attachment '{}' is not valid base64 data", attachment.name))?;
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(format!(
                "Attachment '{}' is larger than 20 MB",
                attachment.name
            ));
        }
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > MAX_ATTACHMENTS_BYTES {
            return Err("Attachments in one message cannot exceed 50 MB in total".into());
        }
        let display_name = Path::new(&attachment.name)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("attachment")
            .to_string();
        let safe_name = safe_attachment_name(&attachment.name, index + 1)?;
        let path = root.join(format!("{stamp}-{safe_name}"));
        fs::write(&path, bytes)
            .map_err(|error| format!("Could not save attachment '{}': {error}", attachment.name))?;
        stored.push(StoredAttachment {
            name: display_name,
            is_image: is_codex_image(&attachment.mime_type, &path),
            path,
        });
    }
    Ok(stored)
}

fn read_session_store(app: &AppHandle) -> CodexSessionStore {
    fs::read_to_string(session_store_path(app))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_session_store(
    app: &AppHandle,
    sessions: &HashMap<String, CodexSessionLink>,
) -> Result<(), String> {
    let path = session_store_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let store = CodexSessionStore {
        sessions: sessions.clone(),
    };
    let text = serde_json::to_string_pretty(&store).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty() || session_id.len() > MAX_SESSION_ID_LEN {
        return Err("Invalid dsh session id".into());
    }
    if !session_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err("Invalid dsh session id".into());
    }
    Ok(())
}

fn validate_short_identifier(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > MAX_MODEL_ID_LEN
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!("Invalid Codex {label}"));
    }
    Ok(())
}

fn validate_collaboration_mode(value: Option<&str>) -> Result<&str, String> {
    let mode = value.unwrap_or("default");
    if matches!(mode, "default" | "plan") {
        Ok(mode)
    } else {
        Err("Codex collaboration mode must be 'default' or 'plan'".into())
    }
}

fn validate_goal_objective(value: &str) -> Result<String, String> {
    let objective = value.trim();
    if objective.is_empty() {
        return Err("Goal objective must not be empty".into());
    }
    if objective.chars().count() > MAX_GOAL_OBJECTIVE_LEN {
        return Err("Goal objective cannot exceed 4,000 characters".into());
    }
    Ok(objective.to_string())
}

fn parse_thread_goal(value: Option<&Value>) -> Result<Option<CodexThreadGoal>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|error| format!("invalid Codex thread goal: {error}")),
    }
}

fn validate_text(name: &str, value: &str, max: usize, allow_empty: bool) -> Result<(), String> {
    if !allow_empty && value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    if value.len() > max {
        return Err(format!("{name} is too large"));
    }
    Ok(())
}

fn validate_cwd(cwd: &str) -> Result<String, String> {
    if cwd.len() > 32_768 {
        return Err("Workspace path is too large".into());
    }
    let path = Path::new(cwd);
    if !path.is_absolute() {
        return Err("Codex workspace must be an absolute path".into());
    }
    let metadata = fs::metadata(path)
        .map_err(|_| "The dsh workspace is no longer available on this machine".to_string())?;
    if !metadata.is_dir() {
        return Err("Codex workspace must be a directory".into());
    }
    Ok(path.to_string_lossy().into_owned())
}

fn clean_title(title: Option<&str>) -> Option<String> {
    title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(120).collect())
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn push_message(messages: &mut Vec<CodexSessionMessage>, message: CodexSessionMessage) {
    if let Some(existing) = messages.iter_mut().find(|item| item.id == message.id) {
        *existing = message;
        return;
    }
    messages.push(message);
    if messages.len() > MAX_SESSION_MESSAGES {
        let overflow = messages.len() - MAX_SESSION_MESSAGES;
        messages.drain(0..overflow);
    }
}

fn push_session_event_locked(
    inner: &mut RuntimeState,
    session_id: &str,
    method: &str,
    params: Value,
) {
    inner.session_event_seq = inner.session_event_seq.saturating_add(1);
    let event = CodexSessionEvent {
        seq: inner.session_event_seq,
        method: method.to_string(),
        params,
    };
    let queue = inner
        .session_events
        .entry(session_id.to_string())
        .or_default();
    queue.push_back(event);
    while queue.len() > MAX_SESSION_EVENTS {
        queue.pop_front();
    }
}

fn session_for_thread(
    sessions: &HashMap<String, CodexSessionLink>,
    thread_id: &str,
) -> Option<String> {
    sessions
        .iter()
        .find(|(_, link)| link.thread_id == thread_id)
        .map(|(session_id, _)| session_id.clone())
}

fn extract_thread_id(params: &Value) -> Option<String> {
    params
        .get("threadId")
        .and_then(Value::as_str)
        .or_else(|| params.pointer("/thread/id").and_then(Value::as_str))
        .or_else(|| params.pointer("/turn/threadId").and_then(Value::as_str))
        .or_else(|| params.pointer("/item/threadId").and_then(Value::as_str))
        .map(str::to_string)
}

fn request_id_key(id: &Value) -> Result<String, String> {
    match id {
        Value::Number(value) => Ok(format!("n:{value}")),
        Value::String(value) if !value.is_empty() && value.len() <= 512 => Ok(format!("s:{value}")),
        _ => Err("Invalid Codex server request id".into()),
    }
}

fn is_session_notification(method: &str) -> bool {
    matches!(
        method,
        "thread/started"
            | "thread/status/changed"
            | "turn/started"
            | "turn/completed"
            | "turn/diff/updated"
            | "turn/plan/updated"
            | "thread/goal/updated"
            | "thread/goal/cleared"
            | "item/started"
            | "item/completed"
            | "item/agentMessage/delta"
            | "item/plan/delta"
            | "item/commandExecution/outputDelta"
            | "item/reasoning/summaryTextDelta"
            | "serverRequest/resolved"
            | "warning"
            | "error"
    )
}

fn sanitize_server_request(method: &str, id: &Value, params: &Value) -> Value {
    json!({
        "requestId": id,
        "kind": if method.contains("commandExecution") { "command" } else { "fileChange" },
        "threadId": params.get("threadId"),
        "turnId": params.get("turnId"),
        "itemId": params.get("itemId"),
        "reason": params.get("reason"),
        "command": params.get("command"),
        "cwd": params.get("cwd"),
        "grantRoot": params.get("grantRoot"),
        "availableDecisions": params.get("availableDecisions")
    })
}

fn sanitize_session_params(method: &str, params: &Value) -> Value {
    if method == "item/agentMessage/delta"
        || method == "item/plan/delta"
        || method == "item/commandExecution/outputDelta"
        || method == "item/reasoning/summaryTextDelta"
    {
        let delta = params
            .get("delta")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .take(32_768)
            .collect::<String>();
        return json!({
            "threadId": params.get("threadId"),
            "turnId": params.get("turnId"),
            "itemId": params.get("itemId"),
            "delta": delta
        });
    }
    let serialized = serde_json::to_string(params).unwrap_or_default();
    if serialized.len() <= 200_000 {
        params.clone()
    } else {
        json!({
            "threadId": params.get("threadId"),
            "turnId": params.get("turnId"),
            "truncated": true,
            "message": "Codex event was too large to display"
        })
    }
}

fn authorize_session_bridge(
    window: &WebviewWindow,
    supervisor: &HarnessSupervisor,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Codex session bridge is only available to the main dsh window".into());
    }
    let expected = supervisor.url().ok_or("DeepSeek Harness is not ready")?;
    let expected = tauri::Url::parse(&expected).map_err(|_| "Invalid dsh origin")?;
    let current = window.url().map_err(|_| "Unable to verify dsh origin")?;
    if current.origin() != expected.origin() {
        return Err("Codex session bridge rejected an untrusted web origin".into());
    }
    Ok(())
}

fn authorize_control_bridge(
    window: &WebviewWindow,
    supervisor: &HarnessSupervisor,
) -> Result<(), String> {
    let current = window.url().map_err(|_| "Unable to verify window origin")?;
    if window.label() == "settings" {
        let bundled = current.scheme() == "tauri"
            || (current.scheme() == "http" && current.host_str() == Some("tauri.localhost"));
        return bundled
            .then_some(())
            .ok_or_else(|| "Codex control bridge rejected an untrusted settings origin".into());
    }
    authorize_session_bridge(window, supervisor)
}

#[tauri::command]
pub fn codex_status(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<CodexStatus, String> {
    authorize_control_bridge(&window, &supervisor)?;
    Ok(manager.refresh_detection(&app))
}

#[tauri::command]
pub fn codex_status_cached(
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<CodexStatus, String> {
    authorize_control_bridge(&window, &supervisor)?;
    Ok(manager.status())
}

#[tauri::command]
pub fn codex_configure(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<(), String> {
    authorize_control_bridge(&window, &supervisor)?;
    manager.configure_async(app);
    Ok(())
}

#[tauri::command]
pub fn codex_login_chatgpt(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<(), String> {
    authorize_control_bridge(&window, &supervisor)?;
    manager.login_async(app);
    Ok(())
}

#[tauri::command]
pub fn codex_logout_chatgpt(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<(), String> {
    authorize_control_bridge(&window, &supervisor)?;
    manager.logout_async(app);
    Ok(())
}

#[tauri::command]
pub fn backend_select(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    backend: String,
) -> Result<(), String> {
    authorize_control_bridge(&window, &supervisor)?;
    match backend.as_str() {
        "codex" => {
            manager.configure_async(app);
            Ok(())
        }
        "dsh" => manager.select_dsh(&app),
        _ => Err("backend must be 'dsh' or 'codex'".into()),
    }
}

#[tauri::command]
pub fn codex_usage_status(
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<CodexUsageSnapshot, String> {
    authorize_control_bridge(&window, &supervisor)?;
    Ok(manager.usage())
}

#[tauri::command]
pub fn codex_usage_refresh(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<(), String> {
    authorize_control_bridge(&window, &supervisor)?;
    manager.refresh_usage_async(app);
    Ok(())
}

#[tauri::command]
pub fn codex_session_poll(
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    session_id: String,
    after_seq: Option<u64>,
) -> Result<CodexSessionSnapshot, String> {
    authorize_session_bridge(&window, &supervisor)?;
    manager.session_snapshot(&session_id, after_seq.unwrap_or(0))
}

#[tauri::command]
pub fn codex_session_index(
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<Vec<CodexSessionIndexEntry>, String> {
    authorize_session_bridge(&window, &supervisor)?;
    Ok(manager.session_index())
}

#[tauri::command]
pub async fn codex_model_catalog(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<CodexModelCatalog, String> {
    authorize_session_bridge(&window, &supervisor)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.model_catalog(&app))
        .await
        .map_err(|error| format!("Codex model worker failed: {error}"))?
}

#[tauri::command]
pub async fn codex_collaboration_mode_catalog(
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
) -> Result<CodexCollaborationModeCatalog, String> {
    authorize_session_bridge(&window, &supervisor)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.collaboration_mode_catalog())
        .await
        .map_err(|error| format!("Codex collaboration-mode worker failed: {error}"))?
}

#[tauri::command]
pub async fn codex_session_send(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    request: CodexSessionSendRequest,
) -> Result<CodexSessionSendResult, String> {
    authorize_session_bridge(&window, &supervisor)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.session_send(&app, request))
        .await
        .map_err(|error| format!("Codex session worker failed: {error}"))?
}

#[tauri::command]
pub async fn codex_session_interrupt(
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    session_id: String,
) -> Result<(), String> {
    authorize_session_bridge(&window, &supervisor)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.session_interrupt(&session_id))
        .await
        .map_err(|error| format!("Codex session worker failed: {error}"))?
}

#[tauri::command]
pub async fn codex_session_goal_get(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    session_id: String,
) -> Result<Option<CodexThreadGoal>, String> {
    authorize_session_bridge(&window, &supervisor)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.session_goal_get(&app, &session_id))
        .await
        .map_err(|error| format!("Codex goal worker failed: {error}"))?
}

#[tauri::command]
pub async fn codex_session_goal_update(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    request: CodexSessionGoalUpdateRequest,
) -> Result<CodexThreadGoal, String> {
    authorize_session_bridge(&window, &supervisor)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.session_goal_update(&app, request))
        .await
        .map_err(|error| format!("Codex goal worker failed: {error}"))?
}

#[tauri::command]
pub async fn codex_session_goal_clear(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    session_id: String,
) -> Result<(), String> {
    authorize_session_bridge(&window, &supervisor)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.session_goal_clear(&app, &session_id))
        .await
        .map_err(|error| format!("Codex goal worker failed: {error}"))?
}

#[tauri::command]
pub fn codex_session_approve(
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    session_id: String,
    request_id: Value,
    decision: String,
) -> Result<(), String> {
    authorize_session_bridge(&window, &supervisor)?;
    manager.session_approve(&session_id, &request_id, &decision)
}

#[tauri::command]
pub fn codex_session_reset(
    app: AppHandle,
    window: WebviewWindow,
    supervisor: State<'_, HarnessSupervisor>,
    manager: State<'_, CodexManager>,
    session_id: String,
) -> Result<(), String> {
    authorize_session_bridge(&window, &supervisor)?;
    manager.session_reset(&app, &session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_cli_version() {
        assert_eq!(
            parse_codex_version("codex-cli 0.147.0\n"),
            Some("0.147.0".into())
        );
        assert_eq!(parse_codex_version(""), None);
    }

    #[test]
    fn pinned_releases_match_verified_date() {
        assert_eq!(CODEX_CLI_VERSION, "0.147.0");
        assert_eq!(DSH_VERSION, "0.1.0-rc.6");
    }

    #[test]
    fn validates_session_adapter_inputs() {
        assert!(validate_session_id("session-123_abc.test:1").is_ok());
        assert!(validate_session_id("").is_err());
        assert!(validate_session_id("../../escape").is_err());
        assert_eq!(clean_title(Some("  hello  ")), Some("hello".into()));
        assert_eq!(clean_title(Some("   ")), None);
        assert_eq!(
            safe_attachment_name("report (final).md", 2).unwrap(),
            "02-report _final_.md"
        );
        assert!(safe_attachment_name("../", 1).is_err());
    }

    #[test]
    fn validates_native_collaboration_modes() {
        assert_eq!(validate_collaboration_mode(None).unwrap(), "default");
        assert_eq!(
            validate_collaboration_mode(Some("default")).unwrap(),
            "default"
        );
        assert_eq!(validate_collaboration_mode(Some("plan")).unwrap(), "plan");
        assert!(validate_collaboration_mode(Some("agent")).is_err());
    }

    #[test]
    fn validates_and_parses_thread_goals() {
        assert_eq!(validate_goal_objective("  ship it  ").unwrap(), "ship it");
        assert!(validate_goal_objective("").is_err());
        assert!(validate_goal_objective(&"x".repeat(MAX_GOAL_OBJECTIVE_LEN + 1)).is_err());

        let goal = parse_thread_goal(Some(&json!({
            "threadId": "thread-1",
            "objective": "Finish native plan mode",
            "status": "active",
            "tokenBudget": 1200,
            "tokensUsed": 25,
            "timeUsedSeconds": 3,
            "createdAt": 1,
            "updatedAt": 2
        })))
        .unwrap()
        .unwrap();
        assert_eq!(goal.objective, "Finish native plan mode");
        assert_eq!(goal.status, "active");
        assert_eq!(goal.token_budget, Some(1200));
        assert_eq!(goal.tokens_used, 25);
    }

    #[test]
    fn session_event_queue_is_bounded() {
        let mut inner = RuntimeState::default();
        for index in 0..(MAX_SESSION_EVENTS + 10) {
            push_session_event_locked(&mut inner, "s1", "test", json!({ "index": index }));
        }
        let queue = inner.session_events.get("s1").unwrap();
        assert_eq!(queue.len(), MAX_SESSION_EVENTS);
        assert_eq!(queue.front().unwrap().seq, 11);
    }
}
