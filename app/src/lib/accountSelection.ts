import type { GitHubIdentity } from "./bindings/GitHubIdentity";
import type { SelectAccountResult } from "./bindings/SelectAccountResult";

export type SelectAccountRequest = {
  identity: GitHubIdentity;
  confirmationNonce: number | null;
};

export async function selectAccountWithConfirmation(
  identity: GitHubIdentity,
  invokeSelection: (
    request: SelectAccountRequest
  ) => Promise<SelectAccountResult>,
  confirmPendingEdits: (pendingEdits: number) => boolean
): Promise<SelectAccountResult | null> {
  let result = await invokeSelection({
    identity,
    confirmationNonce: null,
  });
  if (result.type !== "confirmationRequired") return result;

  if (!confirmPendingEdits(result.pendingEdits)) return null;
  result = await invokeSelection({
    identity,
    confirmationNonce: result.confirmationNonce,
  });
  return result;
}
