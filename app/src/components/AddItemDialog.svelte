<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import { getWorkItemContext } from "$lib/WorkItemContext.svelte";
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

  // After resolving, this holds the existing parent of the item (if any).
  let existingParentId = $state<WorkItemId | null>(null);

  // Whether the user has been shown the reparent dialog and chosen to reparent.
  // null = not yet asked, true = reparent confirmed, false = keep existing
  let reparentChoice = $state<boolean | null>(null);

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
    if (open) prefillFromClipboard();
  });

  function prefillFromClipboard() {
    navigator.clipboard
      .readText()
      .then((text) => {
        const trimmed = text.trim();
        if (trimmed.startsWith("https://github.com/")) {
          url = trimmed;
        }
      })
      .catch(() => {});
  }

  function reset() {
    url = "";
    resolvedId = null;
    existingParentId = null;
    reparentChoice = null;
    error = null;
    resolving = false;
  }

  function handleClose() {
    open = false;
    reset();
  }

  async function resolve() {
    if (!url.trim()) return;

    resolving = true;
    error = null;
    resolvedId = null;
    existingParentId = null;
    reparentChoice = null;

    try {
      const resolved = await context.resolveUrl(url.trim());
      resolvedId = resolved.id;

      // Check if the item is already in the project and has a parent.
      const existing = context.data.workItems[resolved.id];
      if (existing?.data.type === "issue") {
        existingParentId = existing.data.parentId ?? null;
      } else {
        existingParentId = null;
      }
    } catch (e) {
      error = String(e);
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

    // 3. Inherit epic from context: explicit epicId prop overrides, then parent's epic.
    const effectiveEpicId: FieldOptionId | null | undefined =
      epicId !== undefined
        ? epicId
        : doSetParent && parentId
          ? (parentItem?.projectItem.epic ?? null)
          : null;

    if (effectiveEpicId) {
      changes.push({
        workItemId: resolvedId,
        data: { type: "epic", value: effectiveEpicId },
      });
    }

    // 4. Inherit workstream from parent if parent is provided.
    if (doSetParent && parentId && parentItem) {
      const ws = parentItem.projectItem.workstream;
      const workstreamId =
        ws && "loadState" in ws && ws.loadState === "loaded" ? ws.value : null;
      if (workstreamId) {
        changes.push({
          workItemId: resolvedId,
          data: { type: "workstream", value: workstreamId },
        });
      }
    }

    await context.addChanges(changes);
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
    if (!details.open) handleClose();
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
      <div class="text-sm space-y-1">
        <p>
          <span class="font-semibold">ID:</span>
          <span class="font-mono opacity-80">{resolvedId}</span>
        </p>
        {#if alreadyInProject}
          <p class="opacity-70">This item is already in the project.</p>
        {:else}
          <p class="opacity-70">This item will be added to the project.</p>
        {/if}
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
            class="btn rounded px-3 py-1 text-sm"
            onclick={onReparentConfirmed}
          >
            Reparent
          </button>
          <button
            class="btn rounded px-3 py-1 text-sm"
            onclick={onKeepExistingParent}
          >
            Keep existing parent
          </button>
          <button
            class="btn rounded px-3 py-1 text-sm"
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
