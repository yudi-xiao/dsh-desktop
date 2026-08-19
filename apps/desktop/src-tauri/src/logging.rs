//! Small file logger for packaged desktop builds.
//!
//! A GUI-subsystem process has no useful stderr once launched from an
//! installer shortcut, so operational messages are written below the user's
//! app-data directory. Files roll daily and only the current day plus the two
//! preceding calendar days are retained.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::{Days, Local, NaiveDate};

const FILE_PREFIX: &str = "dsh-desktop-";
const FILE_SUFFIX: &str = ".log";
const RETAIN_DAYS: u64 = 3;

struct FileLogger {
    directory: PathBuf,
    current_day: Mutex<NaiveDate>,
}

static LOGGER: OnceLock<FileLogger> = OnceLock::new();

/// Initializes logging at `<app_data_dir>/logs` and returns that directory.
pub fn init(app_data_dir: &Path) -> io::Result<PathBuf> {
    let directory = app_data_dir.join("logs");
    fs::create_dir_all(&directory)?;
    let today = Local::now().date_naive();
    prune(&directory, today)?;

    let _ = LOGGER.set(FileLogger {
        directory: directory.clone(),
        current_day: Mutex::new(today),
    });
    info(
        "desktop",
        format!("starting DSH Desktop {}", env!("CARGO_PKG_VERSION")),
    );
    info("desktop", format!("logs: {}", directory.display()));
    Ok(directory)
}

pub fn info(scope: &str, message: impl AsRef<str>) {
    write_line("INFO", scope, message.as_ref());
}

pub fn warn(scope: &str, message: impl AsRef<str>) {
    write_line("WARN", scope, message.as_ref());
}

pub fn error(scope: &str, message: impl AsRef<str>) {
    write_line("ERROR", scope, message.as_ref());
}

fn write_line(level: &str, scope: &str, message: &str) {
    // Keep developer builds convenient while release builds remain a true GUI
    // process without a console window.
    #[cfg(debug_assertions)]
    eprintln!("[{scope}] {message}");

    let Some(logger) = LOGGER.get() else {
        return;
    };
    let now = Local::now();
    let today = now.date_naive();
    let Ok(mut current_day) = logger.current_day.lock() else {
        return;
    };
    if *current_day != today {
        let _ = prune(&logger.directory, today);
        *current_day = today;
    }

    let path = logger.directory.join(log_file_name(today));
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let safe_message = redact(message);
    let _ = writeln!(
        file,
        "{} [{level}] [{scope}] {safe_message}",
        now.format("%Y-%m-%d %H:%M:%S%.3f %:z")
    );
}

fn redact(message: &str) -> &str {
    let lower = message.to_ascii_lowercase();
    const SENSITIVE_MARKERS: &[&str] = &[
        "authorization:",
        "access_token",
        "refresh_token",
        "id_token",
        "api_key",
        "apikey",
    ];
    if SENSITIVE_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "[redacted potentially sensitive log line]"
    } else {
        message
    }
}

fn prune(directory: &Path, today: NaiveDate) -> io::Result<()> {
    let cutoff = today
        .checked_sub_days(Days::new(RETAIN_DAYS - 1))
        .unwrap_or(NaiveDate::MIN);
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(day) = parse_log_date(&name.to_string_lossy()) else {
            continue;
        };
        if day < cutoff {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn log_file_name(day: NaiveDate) -> String {
    format!("{FILE_PREFIX}{}{FILE_SUFFIX}", day.format("%Y-%m-%d"))
}

fn parse_log_date(name: &str) -> Option<NaiveDate> {
    let value = name.strip_prefix(FILE_PREFIX)?.strip_suffix(FILE_SUFFIX)?;
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_owned_daily_log_files() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        assert_eq!(log_file_name(day), "dsh-desktop-2026-08-17.log");
        assert_eq!(parse_log_date("dsh-desktop-2026-08-17.log"), Some(day));
        assert_eq!(parse_log_date("other-2026-08-17.log"), None);
        assert_eq!(parse_log_date("dsh-desktop-current.log"), None);
    }

    #[test]
    fn redacts_common_secret_bearing_lines() {
        assert_eq!(
            redact("Authorization: Bearer secret"),
            "[redacted potentially sensitive log line]"
        );
        assert_eq!(redact("server ready"), "server ready");
    }

    #[test]
    fn pruning_keeps_exactly_three_calendar_days_and_unrelated_files() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "dsh-desktop-log-test-{}-{nonce}",
            std::process::id(),
        ));
        fs::create_dir_all(&directory).unwrap();
        for offset in 0..=3 {
            let day = today.checked_sub_days(Days::new(offset)).unwrap();
            fs::write(directory.join(log_file_name(day)), b"test").unwrap();
        }
        fs::write(directory.join("keep-me.txt"), b"test").unwrap();

        prune(&directory, today).unwrap();

        for offset in 0..=2 {
            let day = today.checked_sub_days(Days::new(offset)).unwrap();
            assert!(directory.join(log_file_name(day)).exists());
        }
        let expired = today.checked_sub_days(Days::new(3)).unwrap();
        assert!(!directory.join(log_file_name(expired)).exists());
        assert!(directory.join("keep-me.txt").exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
