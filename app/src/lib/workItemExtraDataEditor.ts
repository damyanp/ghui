import type { GitHubIdentity } from "./bindings/GitHubIdentity";

export function canSaveWorkItemExtraDataEditor(
  openedIdentity: GitHubIdentity | null,
  currentIdentity: GitHubIdentity | null,
  openedGeneration: number,
  currentGeneration: number
): boolean {
  return (
    openedIdentity !== null &&
    currentIdentity !== null &&
    openedGeneration === currentGeneration &&
    openedIdentity.host === currentIdentity.host &&
    openedIdentity.login === currentIdentity.login
  );
}
