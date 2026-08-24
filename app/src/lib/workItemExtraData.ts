export function mergePendingExtraData(
  loaded: Record<string, unknown>,
  pending: ReadonlyMap<string, unknown>
): Record<string, unknown> {
  const merged = { ...loaded };
  for (const [id, data] of pending) {
    merged[id] = data;
  }
  return merged;
}
