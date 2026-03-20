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
/// `env_logger::init()`).  Respects the `RUST_LOG` environment variable for
/// level filtering; defaults to `Info` when unset.
pub fn init() {
    WATCHER.get_or_init(|| Mutex::new(None));

    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LevelFilter::Info);

    let logger = Box::new(AppLogger { level });
    log::set_max_level(level);
    log::set_logger(Box::leak(logger)).expect("Failed to set logger");
}

/// Connects the logger to the DataUpdate watcher so that log messages are
/// forwarded to the frontend as `DataUpdate::Log(LogEntry)`.
pub fn set_watcher(watcher: Watcher) {
    if let Some(lock) = WATCHER.get()
        && let Ok(mut guard) = lock.lock()
    {
        *guard = Some(watcher);
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
        if let Some(lock) = WATCHER.get()
            && let Ok(guard) = lock.lock()
            && let Some(watcher) = guard.as_ref()
        {
            let entry = LogEntry {
                timestamp,
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
        // Verify the mapping from log::Level to LogLevel
        let cases = [
            (Level::Error, "Error"),
            (Level::Warn, "Warning"),
            (Level::Info, "Info"),
            (Level::Debug, "Info"),
            (Level::Trace, "Info"),
        ];

        for (level, expected) in cases {
            let log_level = match level {
                Level::Error => LogLevel::Error,
                Level::Warn => LogLevel::Warning,
                _ => LogLevel::Info,
            };
            assert_eq!(format!("{log_level:?}"), expected);
        }
    }
}
