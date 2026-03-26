<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import { getWorkItemContext, recordTelemetry } from "$lib/WorkItemContext.svelte";
  import type { Change } from "$lib/bindings/Change";
  import type { WorkItemId } from "$lib/bindings/WorkItemId";
  import type { FieldOptionId } from "$lib/bindings/FieldOptionId";
  import { LinkIcon } from "@lucide/svelte";

  type Props = {
    open?: boolean;
    /** If set, the added item will be made a child of this work item. */
    parentId?: WorkItemId;
    /** If set, the added item will be placed in this epic. */
    epicId?: FieldOptionId | null;
  };

  let { open = $bindable(false), parentId, epicId }: Props = $props();

  const context = getWorkItemContext();

  let url = $state("");
  let resolving = $state(false);
  let error = $state<string | null>(null);
  let resolvedId = $state<WorkItemId | null>(null);
  let resolvedTitle = $state<string | null>(null);

  // After resolving, this holds the existing parent of the item (if any).
  let existingParentId = $state<WorkItemId | null>(null);

  // Whether the user has been shown the reparent dialog and chosen to reparent.
  // null = not yet asked, true = reparent confirmed, false = keep existing
  let reparentChoice = $state<boolean | null>(null);

  // User-selected epic and workstream (initialised when item is resolved).
  // Empty string means "no selection" (null can't be used as <option value> reliably).
  let selectedEpicId = $state<FieldOptionId | "">("");
  let selectedWorkstreamId = $state<FieldOptionId | "">("");

  const parentItem = $derived(
    parentId ? context.data.workItems[parentId] : undefined
  );

  const existingParentItem = $derived(
    existingParentId ? context.data.workItems[existingParentId] : undefined
  );

  // Whether we need to show the reparent prompt.
  const needsReparentConfirmation = $derived(
    resolvedId !== null &&
      parentId !== undefined &&
      existingParentId !== null &&
      existingParentId !== parentId &&
      reparentChoice === null
  );

  function displayName(id: WorkItemId | undefined): string {
    if (!id) return "?";
    const item = context.data.workItems[id];
    if (!item) return id;
    if (item.resourcePath) {
      // resourcePath has the form /owner/repo/issues/123 — pick owner and number.
      const parts = item.resourcePath.split("/");
      return `${parts.at(-3) ?? "?"}#${parts.at(-1) ?? "?"}`;
    }
    return item.title || id;
  }

  function parentTitle(id: WorkItemId | undefined): string | undefined {
    if (!id) return undefined;
    return context.data.workItems[id]?.title;
  }

  $effect(() => {
    if (open) {
      prefillFromClipboard();
    } else {
      reset();
    }
  });

  function prefillFromClipboard() {
    navigator.clipboard
      .readText()
      .then((text) => {
        const trimmed = text.trim();
        if (!open || url !== "") {
          // Only prefill if the dialog is still open and the user hasn't typed yet.
          return;
        }
        if (trimmed.startsWith("https://github.com/")) {
          url = trimmed;
        }
      })
      .catch(() => {});
  }

  function reset() {
    url = "";
    resolvedId = null;
    resolvedTitle = null;
    existingParentId = null;
    reparentChoice = null;
    error = null;
    resolving = false;
    selectedEpicId = "";
    selectedWorkstreamId = "";
  }

  function handleClose() {
    open = false;
    // reset() is called by the $effect watching `open`
  }

  async function resolve() {
    if (!url.trim()) return;

    resolving = true;
    error = null;
    resolvedId = null;
    resolvedTitle = null;
    existingParentId = null;
    reparentChoice = null;

    try {
      const resolved = await context.resolveUrl(url.trim());
      resolvedId = resolved.id;
      resolvedTitle = resolved.title;

      // Check if the item is already in the project and has a parent.
      const existing = context.data.workItems[resolved.id];
      if (existing?.data.type === "issue") {
        existingParentId = existing.data.parentId ?? null;
      } else {
        existingParentId = null;
      }

      // Initialise epic and workstream selectors from the effective default.
      selectedEpicId =
        epicId !== undefined
          ? (epicId ?? "")
          : (parentItem?.projectItem.epic ?? "");

      const ws = parentItem?.projectItem.workstream;
      selectedWorkstreamId =
        ws && "loadState" in ws && ws.loadState === "loaded"
          ? (ws.value ?? "")
          : "";
    } catch (e) {
      error = e instanceof Error ? e.message : JSON.stringify(e);
    } finally {
      resolving = false;
    }
  }

  async function addItem(doSetParent: boolean) {
    if (!resolvedId) return;

    const changes: Change[] = [];
    const alreadyInProject = !!context.data.workItems[resolvedId];

    // 1. Add to project if needed.
    if (!alreadyInProject) {
      changes.push({
        workItemId: resolvedId,
        data: { type: "addToProject" },
      });
    }

    // 2. Set parent if requested and confirmed.
    if (doSetParent && parentId) {
      changes.push({
        workItemId: resolvedId,
        data: { type: "setParent", value: parentId },
      });
    }

    // 3. Apply user-selected epic (if any).
    if (selectedEpicId) {
      changes.push({
        workItemId: resolvedId,
        data: { type: "epic", value: selectedEpicId },
      });
    }

    // 4. Apply user-selected workstream (if any).
    if (selectedWorkstreamId) {
      changes.push({
        workItemId: resolvedId,
        data: { type: "workstream", value: selectedWorkstreamId },
      });
    }

    await context.addChanges(changes);
    recordTelemetry({ event: "add_item_from_url", has_parent: doSetParent && !!parentId });
    handleClose();
  }

  async function onConfirm() {
    if (!resolvedId) return;
    await addItem(true);
  }

  async function onReparentConfirmed() {
    reparentChoice = true;
    await addItem(true);
  }

  async function onKeepExistingParent() {
    reparentChoice = false;
    await addItem(false);
  }
</script>

<Modal
  open={open}
  contentBase="card bg-surface-100-900 p-4 space-y-4 w-[520px]"
  modal
  onOpenChange={(details) => {
    open = details.open;
  }}
>
  {#snippet content()}
    <header class="flex items-center gap-2">
      <LinkIcon size={18} />
      <span class="font-bold text-lg">Add issue from URL</span>
    </header>

    {#if parentId}
      <div class="text-sm opacity-80">
        Parent: <span class="font-semibold">{displayName(parentId)}</span>
        {#if parentTitle(parentId)}
          — {parentTitle(parentId)}
        {/if}
      </div>
    {/if}

    <div class="flex gap-2">
      <input
        class="flex-1 rounded-lg bg-surface-50-950 px-2 py-1 text-sm"
        type="url"
        placeholder="https://github.com/owner/repo/issues/123"
        bind:value={url}
        disabled={resolving}
        onkeydown={(e) => e.key === "Enter" && resolve()}
      />
      <button
        class="btn rounded px-3 py-1 text-sm"
        onclick={resolve}
        disabled={resolving || !url.trim()}
      >
        {resolving ? "Resolving…" : "Resolve"}
      </button>
    </div>

    {#if error}
      <p class="text-error-500 text-sm">{error}</p>
    {/if}

    {#if resolvedId && !needsReparentConfirmation}
      {@const alreadyInProject = !!context.data.workItems[resolvedId]}
      <div class="text-sm space-y-3">
        <div class="space-y-1">
          {#if resolvedTitle}
            <p class="font-semibold">{resolvedTitle}</p>
          {/if}
          {#if alreadyInProject}
            <p class="opacity-70">This item is already in the project.</p>
          {:else}
            <p class="opacity-70">This item will be added to the project.</p>
          {/if}
        </div>

        <div class="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2">
          <label for="epic-select" class="opacity-70">Epic</label>
          <select
            id="epic-select"
            class="rounded bg-surface-50-950 px-2 py-1 text-sm"
            bind:value={selectedEpicId}
          >
            <option value="">—</option>
            {#each context.data.fields.epic.options as option (option.id)}
              <option value={option.id}>{option.value}</option>
            {/each}
          </select>

          <label for="workstream-select" class="opacity-70">Workstream</label>
          <select
            id="workstream-select"
            class="rounded bg-surface-50-950 px-2 py-1 text-sm"
            bind:value={selectedWorkstreamId}
          >
            <option value="">—</option>
            {#each context.data.fields.workstream.options as option (option.id)}
              <option value={option.id}>{option.value}</option>
            {/each}
          </select>
        </div>
      </div>
    {/if}

    {#if needsReparentConfirmation}
      <div class="border rounded-xl p-3 space-y-2 bg-warning-50-950">
        <p class="text-sm font-semibold">This item already has a parent</p>
        <p class="text-sm">
          Current parent:
          <span class="font-semibold">{displayName(existingParentId!)}</span>
          {#if existingParentItem?.title}
            — {existingParentItem.title}
          {/if}
        </p>
        <p class="text-sm">
          New parent:
          <span class="font-semibold">{displayName(parentId!)}</span>
          {#if parentItem?.title}
            — {parentItem.title}
          {/if}
        </p>
        <div class="flex gap-2 pt-1">
          <button
            class="btn rounded px-3 py-1 text-sm preset-filled-primary-500"
            onclick={onReparentConfirmed}
          >
            Reparent
          </button>
          <button
            class="btn rounded px-3 py-1 text-sm preset-tonal"
            onclick={onKeepExistingParent}
          >
            Keep existing parent
          </button>
          <button
            class="btn rounded px-3 py-1 text-sm preset-tonal"
            onclick={handleClose}
          >
            Cancel
          </button>
        </div>
      </div>
    {/if}

    {#if !needsReparentConfirmation}
      <footer class="flex gap-2 justify-end">
        <button
          class="btn rounded px-3 py-1 text-sm"
          onclick={handleClose}
        >
          Cancel
        </button>
        <button
          class="btn rounded px-3 py-1 text-sm preset-filled-primary-500"
          onclick={onConfirm}
          disabled={!resolvedId}
        >
          Add
        </button>
      </footer>
    {/if}
  {/snippet}
</Modal>
