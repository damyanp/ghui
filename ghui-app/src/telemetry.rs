use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use time::OffsetDateTime;
use ts_rs::TS;

static TELEMETRY_FILE: OnceLock<Mutex<File>> = OnceLock::new();
static SESSION_ID: OnceLock<String> = OnceLock::new();

/// Maximum telemetry file size in bytes (10 MB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// When truncating, keep this fraction of the file (the most recent data).
const KEEP_FRACTION: f64 = 0.75;

/// All telemetry events the app can record.  Each variant carries only the data
/// relevant to that event.  Serialization to JSON happens inside [`record()`],
/// keeping the rest of the codebase free of raw `serde_json::Value`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export)]
pub enum TelemetryEvent {
    SessionStart,
    Refresh,
    Save {
        changes_count: usize,
        duration_ms: u64,
        success: bool,
    },
    Discard {
        changes_count: usize,
    },
    PreviewToggled {
        enabled: bool,
    },
    FilterChanged {
        active_filters: usize,
    },
    FieldChanged {
        field: String,
        value: Option<String>,
    },
    ChangeReverted {
        field: String,
        value: Option<String>,
    },
    Undo,
    Redo,
    Sanitize {
        changes_count: usize,
    },
    ConvertTracked,

    // Frontend-originated events
    ModeSwitched {
        to: String,
    },
    LogPanelToggled {
        open: bool,
    },
    PendingChangesOpened,
    FindDialog,
    ColumnReorder {
        column: String,
    },
    ColumnResize {
        column: String,
    },
    ExpandCollapse {
        action: String,
    },
}

/// Wire format written to the JSONL file.  The `event` tag and associated data
/// come from flattening the [`TelemetryEvent`] enum.
#[derive(Serialize)]
struct TelemetryRecord<'a> {
    ts: String,
    session: &'a str,
    #[serde(flatten)]
    event: &'a TelemetryEvent,
}

/// Initializes the telemetry subsystem.  Call once at application startup.
///
/// Opens (or creates) the telemetry file at [`get_telemetry_file_path()`] in
/// read-write mode and seeks to the end.  Generates a session ID and writes a
/// `session_start` event.
///
/// The file is opened with `write` (not `append`) so that [`truncate_file()`]
/// can seek to the start and rewrite the contents.  Each call to [`record()`]
/// explicitly seeks to the end before writing.
///
/// Safe to call more than once — subsequent calls are no-ops.
pub fn init() {
    if TELEMETRY_FILE.get().is_some() {
        return;
    }

    SESSION_ID.get_or_init(generate_session_id);

    let path = get_telemetry_file_path();
    match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
    {
        Ok(mut file) => {
            // Seek to end so subsequent writes append to existing content.
            // If this fails, we still proceed; telemetry is best-effort.
            let _ = file.seek(SeekFrom::End(0));
            let _ = TELEMETRY_FILE.set(Mutex::new(file));
            record(TelemetryEvent::SessionStart);
        }
        Err(e) => {
            eprintln!("ghui: failed to open telemetry file {path:?}: {e}");
        }
    }
}

/// Records a telemetry event to the JSONL file.
///
/// Each call writes one JSON object per line.  If the file exceeds
/// [`MAX_FILE_SIZE`], older entries are trimmed to keep the file under the
/// limit.
pub fn record(event: TelemetryEvent) {
    let Some(lock) = TELEMETRY_FILE.get() else {
        return;
    };
    let Ok(mut file) = lock.lock() else {
        return;
    };

    let session = SESSION_ID.get().map(|s| s.as_str()).unwrap_or("unknown");

    let entry = TelemetryRecord {
        ts: format_iso_timestamp(),
        session,
        event: &event,
    };

    if let Ok(json) = serde_json::to_string(&entry) {
        // Seek to end before writing — the file is opened with write (not
        // append) so that truncate_file() can seek back to the start.
        let _ = file.seek(SeekFrom::End(0));
        if let Err(e) = writeln!(file, "{json}") {
            eprintln!("ghui: failed to write telemetry: {e}");
        }

        // Check file size and truncate if needed.
        if let Ok(metadata) = file.metadata()
            && metadata.len() > MAX_FILE_SIZE
        {
            truncate_file(&mut file);
        }
    }
}

/// Returns the path to the telemetry file (`~/ghui-telemetry.jsonl`).
///
/// Falls back to the current working directory (or `./ghui-telemetry.jsonl` as
/// a last resort) when no home directory can be determined.
pub fn get_telemetry_file_path() -> PathBuf {
    let base = dirs::home_dir().or_else(|| std::env::current_dir().ok());
    let mut path = base.unwrap_or_else(|| PathBuf::from("."));
    path.push("ghui-telemetry.jsonl");
    path
}

/// Generates a random 8-character hex session ID using basic system entropy.
fn generate_session_id() -> String {
    // Read 4 bytes from the current time's nanoseconds and process ID to
    // produce a simple unique-enough session identifier without pulling in a
    // full random crate.
    let now = OffsetDateTime::now_utc();
    let nanos = now.unix_timestamp_nanos() as u64;
    let pid = std::process::id() as u64;
    let mixed = nanos.wrapping_mul(6364136223846793005).wrapping_add(pid);
    format!("{:016x}", mixed)[..8].to_string()
}

fn format_iso_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond()
    )
}

/// Truncates the file to keep the most recent ~75% of data, discarding older
/// entries at the beginning.  Adjusts the cut point to a newline boundary so
/// we never leave a partial JSON line.
fn truncate_file(file: &mut File) {
    let file_len = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => return,
    };

    let keep_bytes = (file_len as f64 * KEEP_FRACTION) as u64;
    let skip = file_len.saturating_sub(keep_bytes);

    if skip == 0 {
        return;
    }

    // Read the portion we want to keep, starting from `skip`.
    // Find the next newline after `skip` to avoid a partial first line.
    if file.seek(SeekFrom::Start(skip)).is_err() {
        return;
    }

    let mut tail = Vec::new();
    if file.read_to_end(&mut tail).is_err() {
        return;
    }

    // Find the first newline in the tail so we start on a clean line boundary.
    let start = match tail.iter().position(|&b| b == b'\n') {
        Some(pos) => pos + 1,
        None => return, // No newline found; don't truncate.
    };

    if start >= tail.len() {
        return;
    }

    let kept = &tail[start..];

    // Rewrite the file from the beginning with the kept data.
    if file.seek(SeekFrom::Start(0)).is_err() {
        return;
    }
    if file.write_all(kept).is_err() {
        return;
    }

    let new_len = kept.len() as u64;
    let _ = file.set_len(new_len);

    // Seek back to end for future appends.
    let _ = file.seek(SeekFrom::End(0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_format_iso_timestamp_has_expected_format() {
        let ts = format_iso_timestamp();
        // Expected format: YYYY-MM-DDTHH:MM:SS.mmmZ
        assert_eq!(ts.len(), 24);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], ".");
        assert!(ts.ends_with('Z'));
    }

    #[test]
    fn test_generate_session_id_has_expected_length() {
        let id = generate_session_id();
        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_get_telemetry_file_path_has_expected_name() {
        let path = get_telemetry_file_path();
        assert_eq!(path.file_name().unwrap(), "ghui-telemetry.jsonl");
    }

    #[test]
    fn test_truncate_file_keeps_recent_data() {
        let dir = std::env::temp_dir().join("ghui_test_truncate");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.jsonl");

        // Write some lines.
        {
            let mut f = File::create(&path).unwrap();
            for i in 0..100 {
                writeln!(f, "{{\"line\":{i}}}").unwrap();
            }
        }

        // Open in read-write mode and truncate.
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();

        truncate_file(&mut file);

        // Read back — should have fewer lines but all should be valid.
        let mut contents = String::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_string(&mut contents).unwrap();

        let lines: Vec<&str> = contents.lines().collect();
        assert!(!lines.is_empty());
        assert!(lines.len() < 100);

        // Every remaining line should be valid JSON.
        for line in &lines {
            assert!(
                serde_json::from_str::<serde_json::Value>(line).is_ok(),
                "Invalid JSON line: {line}"
            );
        }

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_telemetry_record_with_data() {
        let event = TelemetryEvent::Save {
            changes_count: 3,
            duration_ms: 1200,
            success: true,
        };

        let record = TelemetryRecord {
            ts: "2026-01-01T00:00:00.000Z".to_string(),
            session: "abcd1234",
            event: &event,
        };

        let json = serde_json::to_string(&record).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["ts"], "2026-01-01T00:00:00.000Z");
        assert_eq!(parsed["session"], "abcd1234");
        assert_eq!(parsed["event"], "save");
        assert_eq!(parsed["changes_count"], 3);
        assert_eq!(parsed["duration_ms"], 1200);
        assert_eq!(parsed["success"], true);
    }

    #[test]
    fn test_telemetry_record_without_data() {
        let event = TelemetryEvent::Undo;

        let record = TelemetryRecord {
            ts: "2026-01-01T00:00:00.000Z".to_string(),
            session: "abcd1234",
            event: &event,
        };

        let json = serde_json::to_string(&record).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["event"], "undo");
        assert!(parsed.get("changes_count").is_none());
    }

    /// Exercises the production open-mode: `.write(true).truncate(false)` with
    /// explicit `seek(End)` before each write, plus truncation mid-session.
    /// Verifies that writes after truncation land at the correct position and
    /// produce valid JSONL.
    #[test]
    fn test_write_truncate_write_produces_valid_jsonl() {
        let dir = std::env::temp_dir().join("ghui_test_write_truncate_write");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.jsonl");

        // Ensure a clean slate, then open the same way production does.
        let _ = std::fs::remove_file(&path);
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .unwrap();

        // Phase 1: write 100 lines using seek-to-end (mimics record()).
        for i in 0..100 {
            file.seek(SeekFrom::End(0)).unwrap();
            writeln!(file, "{{\"line\":{i}}}").unwrap();
        }

        // Truncate (drops oldest ~25%).
        truncate_file(&mut file);

        // Phase 2: write 10 more lines after truncation.
        for i in 100..110 {
            file.seek(SeekFrom::End(0)).unwrap();
            writeln!(file, "{{\"line\":{i}}}").unwrap();
        }

        // Read everything back and verify.
        let mut contents = String::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_string(&mut contents).unwrap();

        let lines: Vec<&str> = contents.lines().collect();
        assert!(!lines.is_empty());
        // Should have fewer than 110 total (truncation removed some).
        assert!(lines.len() < 110);

        // Every line must be valid JSON and the post-truncation lines must be
        // present at the end.
        for line in &lines {
            assert!(
                serde_json::from_str::<serde_json::Value>(line).is_ok(),
                "Invalid JSON line: {line}"
            );
        }

        // The last 10 lines must be the ones written after truncation.
        let last_10 = &lines[lines.len().saturating_sub(10)..];
        for (idx, line) in last_10.iter().enumerate() {
            let expected_num = 100 + idx;
            let expected = format!("{{\"line\":{expected_num}}}");
            assert_eq!(*line, expected, "Post-truncation line {idx} mismatch");
        }

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }
}
