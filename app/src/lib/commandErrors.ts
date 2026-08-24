import type { LogEntry } from "./bindings/LogEntry";

export function commandErrorLogEntry(
  operation: string,
  error: unknown,
  timestamp = new Date().toISOString()
): LogEntry {
  const message = error instanceof Error ? error.message : String(error);
  return {
    timestamp,
    level: "error",
    message: `${operation}: ${message}`,
  };
}
