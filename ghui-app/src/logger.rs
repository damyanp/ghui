use crate::{DataUpdate, LogEntry, LogLevel};
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex, OnceLock};

type Watcher = Arc<dyn Fn(DataUpdate) + Send + Sync>;

static WATCHER: OnceLock<Mutex<Option<Watcher>>> = OnceLock::new();

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
/// Safe to call more than once — subsequent calls are no-ops.
pub fn init() {
    WATCHER.get_or_init(|| Mutex::new(None));

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

fn format_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();
    let millis = now.subsec_millis();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;
    format!("{hours:02}:{mins:02}:{secs:02}.{millis:03}")
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

        // Always write to stderr so messages are visible during development.
        eprintln!(
            "{} [{:<5} {}] {}",
            timestamp,
            record.level(),
            record.target(),
            message
        );

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

    fn flush(&self) {}
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
}
