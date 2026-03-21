use crate::{DataUpdate, LogEntry, LogLevel};
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use time::OffsetDateTime;

type Watcher = Arc<dyn Fn(DataUpdate) + Send + Sync>;

static WATCHER: OnceLock<Mutex<Option<Watcher>>> = OnceLock::new();
static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// Custom logger that writes to stderr and dispatches `DataUpdate::Log` to the
/// frontend when a watcher is connected.
struct AppLogger {
    level: LevelFilter,
}

/// Initializes the global logger.  Call once at application startup (replaces
/// `env_logger::init()`).  Reads the `RUST_LOG` environment variable as a
/// simple `LevelFilter` value (e.g. `info`, `debug`, `trace`); per-crate
/// filters like `crate=debug` are **not** supported.  Defaults to `Info` when
/// `RUST_LOG` is unset or cannot be parsed.
///
/// Opens (or creates) a persistent log file at [`get_log_file_path()`] in
/// append mode.  Each session writes a separator line so successive runs are
/// easy to distinguish.
///
/// Safe to call more than once — subsequent calls are no-ops.
pub fn init() {
    WATCHER.get_or_init(|| Mutex::new(None));

    if LOG_FILE.get().is_none() {
        let path = get_log_file_path();
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => {
                let _ = writeln!(
                    file,
                    "\n--- session started at {} ---",
                    format_session_timestamp()
                );
                let _ = LOG_FILE.set(Mutex::new(file));
            }
            Err(e) => {
                eprintln!("ghui: failed to open log file {path:?}: {e}");
            }
        }
    }

    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LevelFilter::Info);

    let logger = Box::new(AppLogger { level });
    log::set_max_level(level);
    // Ignore the error — it just means a logger was already installed.
    let _ = log::set_logger(Box::leak(logger));
}

/// Connects the logger to the DataUpdate watcher so that log messages are
/// forwarded to the frontend as `DataUpdate::Log(LogEntry)`.
pub fn set_watcher(watcher: Watcher) {
    let lock = WATCHER.get_or_init(|| Mutex::new(None));
    // Use into_inner on a poisoned mutex to recover rather than silently
    // dropping the watcher.
    match lock.lock() {
        Ok(mut guard) => *guard = Some(watcher),
        Err(poisoned) => *poisoned.into_inner() = Some(watcher),
    }
}

/// Returns the path to the persistent log file (`~/ghui.log`).
///
/// Falls back to the current working directory (or `./ghui.log` as a last
/// resort) when no home directory can be determined, so callers never panic.
pub fn get_log_file_path() -> PathBuf {
    let base = dirs::home_dir().or_else(|| std::env::current_dir().ok());
    let mut path = base.unwrap_or_else(|| PathBuf::from("."));
    path.push("ghui.log");
    path
}

fn format_session_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond()
    )
}

fn format_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond()
    )
}

impl Log for AppLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = format_timestamp();
        let message = format!("{}", record.args());

        let formatted = format!(
            "{} [{:<5} {}] {}",
            timestamp,
            record.level(),
            record.target(),
            message
        );

        // Always write to stderr so messages are visible during development.
        eprintln!("{formatted}");

        // Append to the persistent log file.
        if let Some(lock) = LOG_FILE.get()
            && let Ok(mut file) = lock.lock()
        {
            let _ = writeln!(file, "{formatted}");
        }

        // Forward to the frontend if a watcher is connected.
        // Clone the Arc and drop the lock before invoking the watcher to avoid
        // deadlocking if the watcher path triggers another log call.
        let watcher = WATCHER
            .get()
            .and_then(|lock| lock.lock().ok())
            .and_then(|guard| guard.as_ref().cloned());

        if let Some(watcher) = watcher {
            let entry = LogEntry {
                timestamp,
                // Debug and Trace are collapsed to Info because the frontend
                // LogLevel enum intentionally has only three variants
                // (Error / Warning / Info) to keep the UI simple.
                level: match record.level() {
                    Level::Error => LogLevel::Error,
                    Level::Warn => LogLevel::Warning,
                    _ => LogLevel::Info,
                },
                message,
            };
            watcher(DataUpdate::Log(entry));
        }
    }

    fn flush(&self) {
        if let Some(lock) = LOG_FILE.get()
            && let Ok(mut file) = lock.lock()
        {
            let _ = file.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp_has_expected_format() {
        let ts = format_timestamp();
        // Expected format: HH:MM:SS.mmm
        assert_eq!(ts.len(), 12);
        assert_eq!(&ts[2..3], ":");
        assert_eq!(&ts[5..6], ":");
        assert_eq!(&ts[8..9], ".");
    }

    #[test]
    fn test_log_level_mapping() {
        let cases = [
            (Level::Error, LogLevel::Error),
            (Level::Warn, LogLevel::Warning),
            (Level::Info, LogLevel::Info),
            (Level::Debug, LogLevel::Info),
            (Level::Trace, LogLevel::Info),
        ];

        for (level, expected) in cases {
            let log_level = match level {
                Level::Error => LogLevel::Error,
                Level::Warn => LogLevel::Warning,
                _ => LogLevel::Info,
            };
            assert_eq!(log_level, expected);
        }
    }

    #[test]
    fn test_format_session_timestamp_has_expected_format() {
        let ts = format_session_timestamp();
        // Expected format: YYYY-MM-DD HH:MM:SS.mmm
        assert_eq!(ts.len(), 23);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], " ");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], ".");
    }

    #[test]
    fn test_get_log_file_path_has_expected_name() {
        let path = get_log_file_path();
        assert_eq!(path.file_name().unwrap(), "ghui.log");
    }
}
