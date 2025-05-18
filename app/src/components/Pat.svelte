<script lang="ts">
  import { LoaderCircle, TriangleAlert } from "@lucide/svelte";
  import { Avatar, Modal } from "@skeletonlabs/skeleton-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  type PatStatus =
    | { type: "set"; login: string; avatarUri: string }
    | { type: "notSet" | "checking" | "broken" };

  let patState = $state<PatStatus>({ type: "checking" });
  $inspect(patState);

  let isOpen = $state(false);

  function update_pat_status(pat_status: PatStatus) {
    patState = pat_status;

    if (patState.type === "set") isOpen = false;
    else if (patState.type != "checking") isOpen = true;
  }

  onMount(() => {
    let unregister: UnlistenFn | null = null;

    listen<PatStatus>("pat-status", (e) => {
      update_pat_status(e.payload);
    }).then((u) => (unregister = u));
    invoke("check_pat_status");

    return () => {
      if (unregister) unregister();
    };
  });

  let pat = $state("");

  async function setPat() {
    isOpen = false;
    patState = { type: "checking" };
    await invoke("set_pat", { pat: pat });
    pat = "";
  }

  async function clearPat() {
    isOpen = false;
    patState = { type: "checking" };
    await invoke("set_pat", { pat: "" });
    pat = "";
  }

  let avatar_uri = $derived.by(() => {
    if (patState.type === "set") return patState.avatarUri;
    else return undefined;
  });

  let displayMode = $derived.by(() => {
    switch (patState.type) {
      case "notSet":
        return "Not Set";
      case "broken":
        return "Invalid";
      case "checking":
        return "";
      default:
        return patState.type;
    }
  });

  let canClose = $derived.by(() => patState.type == "set");
</script>

<button onclick={() => (isOpen = true)}>
  <!--
  For some reason the avatar doesn't get set correctly when the PAT is cleared.
  Inspecting avatar_uri shows that it does get set to 'undefined', but the
  fallback doesn't get shown.  Explicitly setting avatar_uri to a value and then
  to undefined does seem to get noticed.  Using #key makes this work, but it
  shouldn't be necessary.
  -->
  {#key avatar_uri}
    <Avatar
      src={avatar_uri}
      name="unknown"
      size="size-12"
      fallbackBase="bg-error-500"
    >
      <div class="w-full h-full flex items-center justify-center">
        {#if patState.type === "checking"}
          <LoaderCircle class="animate-spin" size={32} />
        {:else}
          <TriangleAlert class="text-error-500" size={32} />
        {/if}
      </div>
    </Avatar>
  {/key}
</button>

<Modal
  open={isOpen}
  contentBase="card bg-primary-50-950 p-4 space-y-4 max-w-[640px]"
  modal
  onOpenChange={(details) => {
    if (!details.open && canClose) isOpen = false;
  }}
>
  {#snippet content()}
    <header>
      <p class="font-bold text-xl text-center">
        Personal Access Token
        {displayMode}
      </p>
    </header>
    <article>
      <p class="m-4">
        This app needs access to github. Although we'd do a proper github login
        flow, the llvm project requires github apps to be installed, which isn't
        great for experiments. Instead we use a PAT. This means that as far as
        github is concerned, this app is you. When you generate the PAT you can
        control what it has access to.
      </p>

      <p class="m-4">
        <a
          target="_blank"
          href="https://github.com/settings/tokens"
          class="anchor">Generate PAT here</a
        >. Tokens (classic) with access to projects seem to work well.
      </p>

      <form class="w-full space-y-4 space-x-1">
        <input
          type="password"
          class="input border-primary-500 bg-primary-100-900"
          placeholder="Paste PAT here"
          bind:value={pat}
        />
        <button
          class="btn preset-filled-success-500"
          disabled={pat.length == 0}
          onclick={setPat}>Set</button
        >
        <button
          class="btn preset-filled-warning-500"
          disabled={patState.type === "notSet"}
          onclick={clearPat}>Clear</button
        >
      </form>
    </article>
  {/snippet}
</Modal>
