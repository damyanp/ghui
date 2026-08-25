import { describe, expect, it, vi } from "vitest";
import type { AccountState } from "./bindings/AccountState";
import type { GitHubIdentity } from "./bindings/GitHubIdentity";
import type { SelectAccountResult } from "./bindings/SelectAccountResult";
import {
  selectAccountWithConfirmation,
  type SelectAccountRequest,
} from "./accountSelection";

const identity: GitHubIdentity = { host: "github.com", login: "target" };
const selectedState: AccountState = {
  selected: null,
  busy: false,
  pendingEdits: 0,
  accountGeneration: 0,
};
const selected: SelectAccountResult = {
  type: "selected",
  state: selectedState,
};

function confirmation(nonce: number): SelectAccountResult {
  return {
    type: "confirmationRequired",
    pendingEdits: 2,
    confirmationNonce: nonce,
  };
}

describe("selectAccountWithConfirmation", () => {
  it("selects normally without prompting when no confirmation is required", async () => {
    const invokeSelection = vi.fn(async () => selected);
    const confirmPendingEdits = vi.fn(() => true);

    await expect(
      selectAccountWithConfirmation(
        identity,
        invokeSelection,
        confirmPendingEdits
      )
    ).resolves.toEqual(selected);
    expect(invokeSelection).toHaveBeenCalledWith({
      identity,
      confirmationNonce: null,
    });
    expect(confirmPendingEdits).not.toHaveBeenCalled();
  });

  it("propagates a stale confirmation when edits change during the prompt", async () => {
    let editsChanged = false;
    const invokeSelection = vi.fn(
      async (request: SelectAccountRequest): Promise<SelectAccountResult> => {
        if (request.confirmationNonce === null) return confirmation(41);
        if (editsChanged) throw new Error("confirmation is stale");
        return selected;
      }
    );

    await expect(
      selectAccountWithConfirmation(identity, invokeSelection, () => {
        editsChanged = true;
        return true;
      })
    ).rejects.toThrow("confirmation is stale");
    expect(invokeSelection).toHaveBeenLastCalledWith({
      identity,
      confirmationNonce: 41,
    });
  });

  it("propagates a stale confirmation when the account changes during the prompt", async () => {
    let accountChanged = false;
    const invokeSelection = vi.fn(
      async (request: SelectAccountRequest): Promise<SelectAccountResult> => {
        if (request.confirmationNonce === null) return confirmation(52);
        if (accountChanged) throw new Error("confirmation is stale");
        return selected;
      }
    );

    await expect(
      selectAccountWithConfirmation(identity, invokeSelection, () => {
        accountChanged = true;
        return true;
      })
    ).rejects.toThrow("confirmation is stale");
  });

  it("never replays a consumed nonce on a later selection attempt", async () => {
    const responses = [
      confirmation(61),
      selected,
      confirmation(62),
      selected,
    ];
    const requests: SelectAccountRequest[] = [];
    const invokeSelection = async (request: SelectAccountRequest) => {
      requests.push(request);
      const response = responses.shift();
      if (!response) throw new Error("unexpected request");
      return response;
    };

    await selectAccountWithConfirmation(identity, invokeSelection, () => true);
    await selectAccountWithConfirmation(identity, invokeSelection, () => true);

    expect(requests.map((request) => request.confirmationNonce)).toEqual([
      null,
      61,
      null,
      62,
    ]);
  });
});
