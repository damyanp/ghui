<script lang="ts">
  import {
    CheckCircle2,
    Clipboard,
    LoaderCircle,
    RefreshCw,
    TriangleAlert,
  } from "@lucide/svelte";
  import { Avatar } from "@skeletonlabs/skeleton-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount, tick } from "svelte";
  import Modal from "./Modal.svelte";
  import {
    accountSelectionWarning,
    accountStatusLabel,
    canSelectAccount,
    edgeSelectableIndex,
    nextSelectableIndex,
    type AccountList,
    type AccountState,
    type GitHubAccount,
    type SelectAccountResult,
  } from "./accountList";
  import { selectAccountWithConfirmation } from "$lib/accountSelection";

  let {
    disabled = false,
    selecting = $bindable(false),
  }: { disabled?: boolean; selecting?: boolean } = $props();

  let accountState = $state<AccountState>({
    selected: null,
    busy: false,
    pendingEdits: 0,
    accountGeneration: 0,
  });
  let accounts = $state<AccountList | null>(null);
  let isOpen = $state(false);
  let enumerating = $state(false);
  let selectingLogin = $state<string | null>(null);
  let selectionError = $state<string | null>(null);
  let focusedIndex = $state(-1);
  let requestSerial = 0;

  const selectionDisabled = $derived(disabled || accountState.busy);
  const selected = $derived(accountState.selected);
  const selectedScopeAccount = $derived(
    accounts?.accounts.find(
      (account) =>
        account.selected && account.readiness === "missingProjectScope"
    ) ?? null
  );

  onMount(() => {
    void initialize();
  });

  async function initialize(): Promise<void> {
    try {
      accountState = await invoke<AccountState>("get_account_state");
    } catch (error) {
      selectionError = errorMessage(error);
    }
  }

  async function openAccountList(): Promise<void> {
    if (isOpen) return;
    isOpen = true;
    await refreshAccounts();
  }

  async function refreshAccounts(): Promise<void> {
    if (enumerating) return;
    const request = ++requestSerial;
    enumerating = true;
    selectionError = null;
    try {
      const next = await invoke<AccountList>("list_accounts");
      if (request !== requestSerial) return;
      accountState = await invoke<AccountState>("get_account_state");
      if (request !== requestSerial) return;
      accounts = next;
      focusedIndex = next.accounts.findIndex((account) => account.selected);
      if (focusedIndex < 0) {
        focusedIndex = edgeSelectableIndex(next.accounts, "first");
      }
    } catch (error) {
      if (request === requestSerial) selectionError = errorMessage(error);
    } finally {
      if (request === requestSerial) enumerating = false;
    }
  }

  async function selectAccount(account: GitHubAccount): Promise<void> {
    if (!canSelectAccount(account, selectionDisabled) || selectingLogin) return;

    requestSerial++;
    selectingLogin = account.identity.login;
    selecting = true;
    selectionError = null;
    try {
      const result = await selectAccountWithConfirmation(
        account.identity,
        (request) =>
          invoke<SelectAccountResult>("select_account", request),
        (count) =>
          window.confirm(
          `Switch GitHub accounts with ${count} pending ${
            count === 1 ? "edit" : "edits"
          }? The edits will be re-applied as ${account.identity.login} and may fail.`
          )
      );
      if (!result) return;

      if (result.type === "selected") {
        accountState = result.state;
        if (accounts) {
          accounts = {
            ...accounts,
            accounts: accounts.accounts.map((candidate) => ({
              ...candidate,
              selected:
                candidate.identity.host === account.identity.host &&
                candidate.identity.login === account.identity.login,
            })),
          };
        }
        isOpen = false;
      }
    } catch (error) {
      selectionError = errorMessage(error);
    } finally {
      selectingLogin = null;
      selecting = false;
    }
  }

  async function focusOption(index: number): Promise<void> {
    if (index < 0) return;
    focusedIndex = index;
    await tick();
    document.getElementById(`github-account-${index}`)?.focus();
  }

  function onListKeydown(event: KeyboardEvent): void {
    if (!accounts) return;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        void focusOption(
          nextSelectableIndex(accounts.accounts, focusedIndex, 1)
        );
        break;
      case "ArrowUp":
        event.preventDefault();
        void focusOption(
          nextSelectableIndex(accounts.accounts, focusedIndex, -1)
        );
        break;
      case "Home":
        event.preventDefault();
        void focusOption(edgeSelectableIndex(accounts.accounts, "first"));
        break;
      case "End":
        event.preventDefault();
        void focusOption(edgeSelectableIndex(accounts.accounts, "last"));
        break;
      case "Enter":
      case " ":
        if (focusedIndex >= 0) {
          event.preventDefault();
          const account = accounts.accounts[focusedIndex];
          if (account) void selectAccount(account);
        }
        break;
    }
  }

  async function copyScopeRemediation(account: GitHubAccount): Promise<void> {
    const login = account.identity.login;
    await writeText(
      `gh auth switch --hostname github.com --user ${login}\n` +
        "gh auth refresh --hostname github.com -s project"
    );
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }
</script>

<button
  onclick={() => void openAccountList()}
  title={selected
    ? `GitHub account used by ghui: ${selected.identity.login}`
    : "Select the GitHub account ghui will use"}
  aria-label={selected
    ? `GitHub account ${selected.identity.login}. Open account selection.`
    : "No GitHub account selected. Open account selection."}
>
  <Avatar class="size-12">
    {#if selected}
      <Avatar.Image src={selected.avatarUri} alt={selected.identity.login} />
    {/if}
    <Avatar.Fallback class={selected ? "bg-primary-500" : "bg-error-500"}>
      <div class="w-full h-full flex items-center justify-center">
        {#if selected}
          <span class="font-bold text-white">
            {selected.identity.login.slice(0, 1).toUpperCase()}
          </span>
        {:else}
          <TriangleAlert class="text-white" size={32} />
        {/if}
      </div>
    </Avatar.Fallback>
  </Avatar>
</button>

<Modal
  open={isOpen}
  contentBase="card bg-primary-50-950 p-4 space-y-4 w-full max-w-[680px] max-h-[85vh] overflow-y-auto"
  modal
  onOpenChange={(details) => {
    isOpen = details.open;
  }}
>
  {#snippet content()}
    <header class="space-y-1">
      <p class="font-bold text-xl text-center">GitHub account</p>
      <p class="text-center text-sm opacity-80">
        Choose the stored <code>github.com</code> account ghui will use. This
        does not change the account active in <code>gh</code>.
      </p>
    </header>

    {#if selected}
      <p class="text-sm">
        ghui is configured to use <strong>{selected.identity.login}</strong>.
        {#if selected.state === "tokenMissing"}
          Its stored token is unavailable, so network actions are blocked until
          another account is selected.
        {:else if selected.state !== "unverified"}
          The credential could not be verified ({selected.state}).
        {/if}
      </p>
    {:else}
      <p class="text-sm">
        No account is selected. Cached data remains viewable, but GitHub network
        actions are blocked.
      </p>
    {/if}

    {#if enumerating}
      <div class="flex items-center justify-center gap-2 p-6" role="status">
        <LoaderCircle class="animate-spin" size={24} />
        <span>Checking stored accounts…</span>
      </div>
    {:else if accounts}
      {#if accounts.state === "ghMissing"}
        <p role="alert">
          GitHub CLI was not found. Install it from
          <a class="anchor" href="https://cli.github.com/" target="_blank"
            >cli.github.com</a
          >, then run <code>gh auth login</code>.
        </p>
      {:else if accounts.state === "ghTooOld"}
        <p role="alert">
          This version of GitHub CLI is too old to list accounts. Update
          <code>gh</code> and re-check.
        </p>
      {:else if accounts.state === "noAccounts"}
        <p role="status">
          No stored <code>github.com</code> accounts were found. Run
          <code>gh auth login</code>, then re-check.
        </p>
      {:else if accounts.state === "timeout"}
        <p role="alert">
          Account enumeration timed out. Check connectivity and re-check.
        </p>
      {:else if accounts.state === "error"}
        <p role="alert">
          Account enumeration failed. {accounts.message ?? ""}
        </p>
      {/if}

      {#if accounts.accounts.length > 0}
        <div
          role="listbox"
          aria-label="Stored GitHub accounts"
          class="space-y-2"
          tabindex="-1"
        >
          {#each accounts.accounts as account, index (account.identity.host + account.identity.login)}
            {@const selectable = canSelectAccount(account, selectionDisabled)}
            {@const warning = accountSelectionWarning(account)}
            <div
              id={`github-account-${index}`}
              role="option"
              aria-selected={account.selected}
              aria-disabled={!selectable}
              tabindex={focusedIndex === index ? 0 : -1}
              class={[
                "rounded-container border p-3 flex gap-3 items-center",
                account.selected ? "border-primary-500 bg-primary-100-900" : "",
                selectable
                  ? "cursor-pointer hover:bg-surface-100-900"
                  : "opacity-60 cursor-not-allowed",
              ]}
              onclick={() => void selectAccount(account)}
              onfocus={() => (focusedIndex = index)}
              onkeydown={onListKeydown}
            >
              <Avatar class="size-10 shrink-0">
                {#if account.avatarUri}
                  <Avatar.Image
                    src={account.avatarUri}
                    alt={account.identity.login}
                  />
                {/if}
                <Avatar.Fallback class="bg-surface-300-700">
                  {account.identity.login.slice(0, 1).toUpperCase()}
                </Avatar.Fallback>
              </Avatar>
              <div class="min-w-0 grow">
                <div class="flex items-center gap-2">
                  <strong>{account.identity.login}</strong>
                  {#if account.selected}
                    <span class="badge preset-filled-primary-500">Selected</span>
                  {/if}
                </div>
                <div class="text-sm flex items-center gap-1">
                  {#if account.readiness === "ready"}
                    <CheckCircle2 size={16} class="text-success-600-400" />
                  {:else}
                    <TriangleAlert size={16} class="text-warning-600-400" />
                  {/if}
                  <span>{accountStatusLabel(account.readiness)}</span>
                </div>
                {#if warning || account.detail}
                  <p class="text-xs opacity-80 mt-1">
                    {warning ?? account.detail}
                  </p>
                {/if}
              </div>
              {#if selectingLogin === account.identity.login}
                <LoaderCircle class="animate-spin shrink-0" size={20} />
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/if}

    {#if selectedScopeAccount}
      <aside class="preset-tonal-warning p-3 rounded-container space-y-2">
        <p>
          <code>gh auth refresh</code> cannot target a user. Temporarily switch
          <code>gh</code> to <strong>{selectedScopeAccount.identity.login}</strong>,
          refresh the project scope, then optionally switch it back. ghui's
          selected account will not change.
        </p>
        <button
          class="btn preset-tonal"
          onclick={() => void copyScopeRemediation(selectedScopeAccount)}
        >
          <Clipboard size={16} /> Copy commands
        </button>
      </aside>
    {/if}

    {#if selectionDisabled}
      <p class="text-sm text-warning-700-300" role="status">
        Account selection is disabled while ghui is busy.
      </p>
    {/if}
    {#if selectionError}
      <p class="text-error-700-300" role="alert">{selectionError}</p>
    {/if}

    <footer class="flex justify-end gap-2">
      <button
        class="btn preset-tonal"
        disabled={enumerating || selectingLogin !== null}
        onclick={() => void refreshAccounts()}
      >
        <RefreshCw size={16} class={enumerating ? "animate-spin" : ""} />
        Re-check
      </button>
      <button class="btn preset-filled-primary-500" onclick={() => (isOpen = false)}>
        Close
      </button>
    </footer>
  {/snippet}
</Modal>
