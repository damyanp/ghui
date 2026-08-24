import { describe, expect, it } from "vitest";
import {
  accountSelectionWarning,
  accountStatusLabel,
  canSelectAccount,
  edgeSelectableIndex,
  nextSelectableIndex,
  type AccountReadiness,
  type GitHubAccount,
} from "./accountList";

function account(
  login: string,
  readiness: AccountReadiness,
  selectable = true
): GitHubAccount {
  return {
    identity: { host: "github.com", login },
    avatarUri: null,
    source: readiness === "environmentOnly" ? "environment" : "stored",
    readiness,
    identityVerified: readiness === "ready",
    selected: false,
    selectable,
    detail: null,
  };
}

describe("account list presentation", () => {
  it("provides a label for every backend readiness state", () => {
    const states: AccountReadiness[] = [
      "ready",
      "unverified",
      "missingProjectScope",
      "noProjectAccess",
      "offline",
      "timeout",
      "invalidCredential",
      "credentialMismatch",
      "tokenMissing",
      "environmentOnly",
      "error",
    ];

    for (const state of states) {
      expect(accountStatusLabel(state)).not.toBe("");
    }
  });

  it("keeps access and verification warnings selectable", () => {
    for (const state of [
      "missingProjectScope",
      "noProjectAccess",
      "offline",
      "timeout",
      "error",
    ] satisfies AccountReadiness[]) {
      const candidate = account("octocat", state);
      expect(canSelectAccount(candidate, false)).toBe(true);
      expect(accountSelectionWarning(candidate)).not.toBeNull();
    }
  });

  it("disables environment-only and missing-token accounts", () => {
    expect(
      canSelectAccount(account("env", "environmentOnly", false), false)
    ).toBe(false);
    expect(
      canSelectAccount(account("missing", "tokenMissing", false), false)
    ).toBe(false);
  });

  it("disables every account while the app is busy", () => {
    expect(canSelectAccount(account("octocat", "ready"), true)).toBe(false);
  });
});

describe("account list keyboard navigation", () => {
  const accounts = [
    account("disabled", "environmentOnly", false),
    account("first", "ready"),
    account("second", "offline"),
  ];

  it("wraps and skips disabled rows", () => {
    expect(nextSelectableIndex(accounts, 2, 1)).toBe(1);
    expect(nextSelectableIndex(accounts, 1, -1)).toBe(2);
  });

  it("finds the first and last selectable rows", () => {
    expect(edgeSelectableIndex(accounts, "first")).toBe(1);
    expect(edgeSelectableIndex(accounts, "last")).toBe(2);
  });

  it("returns no index when every row is disabled", () => {
    const disabled = [account("env", "environmentOnly", false)];
    expect(nextSelectableIndex(disabled, 0, 1)).toBe(-1);
    expect(edgeSelectableIndex(disabled, "first")).toBe(-1);
  });
});
