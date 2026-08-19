//! Sanitized Codex account-usage state shared with the dsh web process.
//!
//! Only display-safe account metadata and aggregate usage values are written to
//! disk. OAuth tokens, API keys, app-server messages, and approval payloads
//! never cross this boundary.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsageSnapshot {
    pub backend: String,
    pub phase: String,
    pub cli_version: Option<String>,
    pub required_cli_version: String,
    pub app_server_running: bool,
    pub auth_mode: Option<String>,
    pub managed_install: bool,
    pub state: String,
    pub plan_type: Option<String>,
    pub email: Option<String>,
    pub updated_at: u64,
    pub buckets: Vec<CodexRateLimitBucket>,
    pub credits: Option<CodexCredits>,
    pub reset_credits: Option<CodexResetCredits>,
    pub account_usage: Option<CodexAccountUsage>,
    pub thread_usage: BTreeMap<String, CodexThreadUsage>,
    pub error: Option<String>,
}

impl Default for CodexUsageSnapshot {
    fn default() -> Self {
        Self {
            backend: "dsh".into(),
            phase: "idle".into(),
            cli_version: None,
            required_cli_version: String::new(),
            app_server_running: false,
            auth_mode: None,
            managed_install: false,
            state: "unavailable".into(),
            plan_type: None,
            email: None,
            updated_at: now_millis(),
            buckets: Vec::new(),
            credits: None,
            reset_credits: None,
            account_usage: None,
            thread_usage: BTreeMap::new(),
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitBucket {
    pub id: String,
    pub name: Option<String>,
    pub plan_type: Option<String>,
    pub primary: Option<CodexRateLimitWindow>,
    pub secondary: Option<CodexRateLimitWindow>,
    pub reached_type: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitWindow {
    pub used_percent: i64,
    /// Derived display value. The UI labels it as approximate because the
    /// service reports only usedPercent, not an absolute remaining count.
    pub remaining_percent: i64,
    pub resets_at: Option<i64>,
    pub window_duration_mins: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCredits {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexResetCredits {
    pub available_count: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAccountUsage {
    pub lifetime_tokens: Option<u64>,
    pub peak_daily_tokens: Option<u64>,
    pub longest_running_turn_sec: Option<u64>,
    pub current_streak_days: Option<u64>,
    pub longest_streak_days: Option<u64>,
    pub daily_buckets: Vec<CodexDailyUsage>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexDailyUsage {
    pub start_date: String,
    pub tokens: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexThreadUsage {
    pub turn_id: String,
    pub total: CodexTokenUsage,
    pub last: CodexTokenUsage,
    pub model_context_window: Option<u64>,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexTokenUsage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
}

impl CodexUsageSnapshot {
    pub fn set_runtime(
        &mut self,
        backend: String,
        phase: String,
        cli_version: Option<String>,
        required_cli_version: String,
        app_server_running: bool,
        auth_mode: Option<String>,
        managed_install: bool,
    ) {
        self.backend = backend;
        self.phase = phase;
        self.cli_version = cli_version;
        self.required_cli_version = required_cli_version;
        self.app_server_running = app_server_running;
        self.auth_mode = auth_mode;
        self.managed_install = managed_install;
    }

    pub fn set_loading(&mut self) {
        self.state = "loading".into();
        self.error = None;
        self.touch();
    }

    pub fn set_unavailable(&mut self, error: Option<String>) {
        self.state = "unavailable".into();
        self.error = error;
        self.touch();
    }

    pub fn set_account(&mut self, account: &Value) {
        self.plan_type = account
            .pointer("/account/planType")
            .and_then(Value::as_str)
            .map(str::to_string);
        self.email = account
            .pointer("/account/email")
            .and_then(Value::as_str)
            .map(str::to_string);
        let auth_type = account.pointer("/account/type").and_then(Value::as_str);
        self.state = if auth_type == Some("chatgpt") {
            "loading".into()
        } else {
            "unauthenticated".into()
        };
        self.error = None;
        self.touch();
    }

    pub fn set_plan_type(&mut self, plan_type: Option<String>) {
        self.plan_type = plan_type;
        self.touch();
    }

    pub fn apply_rate_limits(&mut self, response: &Value) {
        let mut buckets = Vec::new();
        if let Some(by_id) = response
            .get("rateLimitsByLimitId")
            .and_then(Value::as_object)
        {
            for (id, value) in by_id {
                if let Some(bucket) = parse_bucket(value, Some(id)) {
                    buckets.push(bucket);
                }
            }
        }
        if buckets.is_empty() {
            if let Some(bucket) = response
                .get("rateLimits")
                .and_then(|value| parse_bucket(value, None))
            {
                buckets.push(bucket);
            }
        }
        buckets.sort_by(|left, right| left.id.cmp(&right.id));
        self.credits = buckets.iter().find_map(|bucket| {
            response
                .get("rateLimitsByLimitId")
                .and_then(|value| value.get(&bucket.id))
                .or_else(|| response.get("rateLimits"))
                .and_then(parse_credits)
        });
        self.reset_credits = response
            .pointer("/rateLimitResetCredits/availableCount")
            .and_then(Value::as_u64)
            .map(|available_count| CodexResetCredits { available_count });
        if self.plan_type.is_none() {
            self.plan_type = buckets.iter().find_map(|bucket| bucket.plan_type.clone());
        }
        self.buckets = buckets;
        self.state = "ready".into();
        self.error = None;
        self.touch();
    }

    pub fn apply_rate_limit_notification(&mut self, params: &Value) {
        let Some(bucket) = params
            .get("rateLimits")
            .and_then(|value| parse_bucket(value, None))
        else {
            return;
        };
        if let Some(existing) = self.buckets.iter_mut().find(|item| item.id == bucket.id) {
            *existing = bucket;
        } else {
            self.buckets.push(bucket);
            self.buckets.sort_by(|left, right| left.id.cmp(&right.id));
        }
        self.state = "ready".into();
        self.error = None;
        self.touch();
    }

    pub fn apply_account_usage(&mut self, response: &Value) {
        let summary = response.get("summary").unwrap_or(&Value::Null);
        let daily_buckets = response
            .get("dailyUsageBuckets")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        Some(CodexDailyUsage {
                            start_date: item.get("startDate")?.as_str()?.to_string(),
                            tokens: item.get("tokens")?.as_u64()?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.account_usage = Some(CodexAccountUsage {
            lifetime_tokens: summary.get("lifetimeTokens").and_then(Value::as_u64),
            peak_daily_tokens: summary.get("peakDailyTokens").and_then(Value::as_u64),
            longest_running_turn_sec: summary.get("longestRunningTurnSec").and_then(Value::as_u64),
            current_streak_days: summary.get("currentStreakDays").and_then(Value::as_u64),
            longest_streak_days: summary.get("longestStreakDays").and_then(Value::as_u64),
            daily_buckets,
        });
        self.touch();
    }

    pub fn apply_thread_usage(&mut self, params: &Value) {
        let Some(thread_id) = params.get("threadId").and_then(Value::as_str) else {
            return;
        };
        let Some(turn_id) = params.get("turnId").and_then(Value::as_str) else {
            return;
        };
        let Some(usage) = params.get("tokenUsage") else {
            return;
        };
        let Some(total) = usage.get("total").and_then(parse_token_usage) else {
            return;
        };
        let Some(last) = usage.get("last").and_then(parse_token_usage) else {
            return;
        };
        self.thread_usage.insert(
            thread_id.to_string(),
            CodexThreadUsage {
                turn_id: turn_id.to_string(),
                total,
                last,
                model_context_window: usage.get("modelContextWindow").and_then(Value::as_u64),
                updated_at: now_millis(),
            },
        );
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = now_millis();
    }
}

fn parse_bucket(value: &Value, fallback_id: Option<&str>) -> Option<CodexRateLimitBucket> {
    let object = value.as_object()?;
    let id = object
        .get("limitId")
        .and_then(Value::as_str)
        .or(fallback_id)
        .unwrap_or("codex")
        .to_string();
    Some(CodexRateLimitBucket {
        id,
        name: object
            .get("limitName")
            .and_then(Value::as_str)
            .map(str::to_string),
        plan_type: object
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_string),
        primary: object.get("primary").and_then(parse_window),
        secondary: object.get("secondary").and_then(parse_window),
        reached_type: object
            .get("rateLimitReachedType")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_window(value: &Value) -> Option<CodexRateLimitWindow> {
    let used_percent = value.get("usedPercent")?.as_i64()?.clamp(0, 100);
    Some(CodexRateLimitWindow {
        used_percent,
        remaining_percent: 100 - used_percent,
        resets_at: value.get("resetsAt").and_then(Value::as_i64),
        window_duration_mins: value.get("windowDurationMins").and_then(Value::as_i64),
    })
}

fn parse_credits(value: &Value) -> Option<CodexCredits> {
    let credits = value.get("credits")?.as_object()?;
    Some(CodexCredits {
        has_credits: credits.get("hasCredits")?.as_bool()?,
        unlimited: credits.get("unlimited")?.as_bool()?,
        balance: credits
            .get("balance")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_token_usage(value: &Value) -> Option<CodexTokenUsage> {
    Some(CodexTokenUsage {
        input_tokens: value.get("inputTokens")?.as_u64()?,
        cached_input_tokens: value.get("cachedInputTokens")?.as_u64()?,
        output_tokens: value.get("outputTokens")?.as_u64()?,
        reasoning_output_tokens: value.get("reasoningOutputTokens")?.as_u64()?,
        total_tokens: value.get("totalTokens")?.as_u64()?,
    })
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub fn snapshot_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|path| path.join("codex-usage.json"))
}

pub fn publish_snapshot(app: &AppHandle, snapshot: &CodexUsageSnapshot) -> Result<(), String> {
    let path = snapshot_path(app).ok_or("application data directory is unavailable")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string(snapshot).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())?;
    let _ = app.emit("codex-usage-updated", snapshot.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_rate_limit_windows_and_credits() {
        let mut snapshot = CodexUsageSnapshot::default();
        snapshot.apply_rate_limits(&json!({
            "rateLimits": {
                "limitId": "codex",
                "planType": "pro",
                "primary": { "usedPercent": 31, "resetsAt": 123, "windowDurationMins": 300 },
                "secondary": null,
                "credits": { "hasCredits": true, "unlimited": false, "balance": "12.50" }
            }
        }));
        assert_eq!(snapshot.state, "ready");
        assert_eq!(snapshot.plan_type.as_deref(), Some("pro"));
        assert_eq!(
            snapshot.buckets[0]
                .primary
                .as_ref()
                .unwrap()
                .remaining_percent,
            69
        );
        assert_eq!(
            snapshot.credits.as_ref().unwrap().balance.as_deref(),
            Some("12.50")
        );
    }

    #[test]
    fn keeps_thread_usage_by_codex_thread_id() {
        let mut snapshot = CodexUsageSnapshot::default();
        snapshot.apply_thread_usage(&json!({
            "threadId": "thread-1",
            "turnId": "turn-1",
            "tokenUsage": {
                "modelContextWindow": 200000,
                "last": { "inputTokens": 5, "cachedInputTokens": 2, "outputTokens": 3, "reasoningOutputTokens": 1, "totalTokens": 8 },
                "total": { "inputTokens": 10, "cachedInputTokens": 4, "outputTokens": 6, "reasoningOutputTokens": 2, "totalTokens": 16 }
            }
        }));
        assert_eq!(snapshot.thread_usage["thread-1"].total.total_tokens, 16);
    }
}
