import type { AccountList } from "$lib/bindings/AccountList";
import type { AccountReadiness } from "$lib/bindings/AccountReadiness";
import type { AccountState } from "$lib/bindings/AccountState";
import type { GitHubAccount } from "$lib/bindings/GitHubAccount";
import type { GitHubIdentity } from "$lib/bindings/GitHubIdentity";
import type { SelectAccountResult } from "$lib/bindings/SelectAccountResult";
import type { SelectedAccount } from "$lib/bindings/SelectedAccount";

export type {
  AccountList,
  AccountReadiness,
  AccountState,
  GitHubAccount,
  GitHubIdentity,
  SelectAccountResult,
  SelectedAccount,
};

const statusLabels: Record<AccountReadiness, string> = {
  ready: "Ready",
  unverified: "Not verified",
  missingProjectScope: "Missing project scope",
  noProjectAccess: "No project access",
  offline: "Offline; not verified",
  timeout: "Check timed out",
  invalidCredential: "Credential problem",
  credentialMismatch: "Credential mismatch",
  tokenMissing: "Stored token unavailable",
  environmentOnly: "Environment token",
  error: "Check failed",
};

export function accountStatusLabel(readiness: AccountReadiness): string {
  return statusLabels[readiness];
}

export function canSelectAccount(
  account: GitHubAccount,
  busy: boolean
): boolean {
  return account.selectable && !busy;
}

export function accountSelectionWarning(
  account: GitHubAccount
): string | null {
  switch (account.readiness) {
    case "missingProjectScope":
      return "This account may fail GitHub Projects operations until its project scope is refreshed.";
    case "noProjectAccess":
      return "This account could not access the configured project.";
    case "offline":
    case "timeout":
    case "unverified":
      return "This account could not be verified, but it can still be selected.";
    case "invalidCredential":
    case "error":
      return "This credential reported an error. Selection is allowed so GitHub can return the authoritative result.";
    case "credentialMismatch":
    case "tokenMissing":
    case "environmentOnly":
    case "ready":
      return null;
  }
}

export function nextSelectableIndex(
  accounts: GitHubAccount[],
  current: number,
  direction: -1 | 1
): number {
  if (accounts.length === 0) return -1;
  for (let offset = 1; offset <= accounts.length; offset++) {
    const candidate =
      (current + direction * offset + accounts.length) % accounts.length;
    if (accounts[candidate]?.selectable) return candidate;
  }
  return -1;
}

export function edgeSelectableIndex(
  accounts: GitHubAccount[],
  edge: "first" | "last"
): number {
  if (edge === "first") return accounts.findIndex((account) => account.selectable);
  return accounts.findLastIndex((account) => account.selectable);
}
