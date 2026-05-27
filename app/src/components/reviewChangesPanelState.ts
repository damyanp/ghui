export type Tab = "changes" | "conflicts";

export function getInitialActiveTab(
  changeCount: number,
  conflictCount: number,
  preferredTab?: Tab
): Tab {
  if (preferredTab === "changes" && changeCount > 0) return "changes";
  if (preferredTab === "conflicts" && conflictCount > 0) return "conflicts";
  if (changeCount === 0 && conflictCount > 0) return "conflicts";
  return "changes";
}
