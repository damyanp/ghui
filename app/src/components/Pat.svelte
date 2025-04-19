<script lang="ts">
  import { TriangleAlert } from "@lucide/svelte";
  import { Avatar, Popover } from "@skeletonlabs/skeleton-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";

  let color = $state("secondary");
  let text = $state("...");
  let hasPat = $state(false);
  let isOpen = $state(true);
  let avatar = $state<string | undefined>(undefined);

  type Status =
    | "NotSet"
    | { Set: { login: string; avatar_uri: string } }
    | "Broken";

  function update_pat_status(pat_status: Status) {
    switch (pat_status) {
      case "NotSet":
        color = "danger";
        text = "Set PAT";
        hasPat = false;
        avatar = undefined;
        break;

      case "Broken":
        color = "danger";
        text = "PAT broken";
        hasPat = true;
        avatar = undefined;
        break;

      default:
        if (typeof pat_status === "object" && "Set" in pat_status) {
          color = "success";
          text = pat_status.Set.login;
          hasPat = true;
          avatar = pat_status.Set.avatar_uri;
        }
        break;
    }
  }

  const unlistenStatus = listen<Status>("pat-status", (e) => {
    console.log(`pat-status: ${JSON.stringify(e.payload)}`);
    update_pat_status(e.payload);
  });
  invoke("check_pat_status");

  onDestroy(() => unlistenStatus.then((u) => u()));

  const propsId = $props.id();
  const buttonId = `${propsId}-button`;

  let pat = $state("");

  async function setPat() {
    await invoke("set_pat", { pat: pat });
    isOpen = false;
    pat = "";
  }

  async function clearPat() {
    await invoke("set_pat", { pat: "" });
    isOpen = false;
    pat = "";
  }

  function setIsOpen(o: boolean) {
    isOpen = o;

    // Make sure that when the popover is opened that the PAT input is empty.
    if (isOpen) pat = "";
  }
</script>

<Popover
  open={isOpen}
  onOpenChange={(e) => (isOpen = e.open)}
  positioning={{ placement: "bottom" }}
  contentBase="card bg-primary-50-950 p-4 space-y-4 max-w-[640px]"
  arrow
  arrowBackground="!bg-primary-50 dark:!bg-primary-950"
>
  {#snippet trigger()}
    <Avatar src={avatar} name={text} size="size-12" fallbackBase="bg-error-500">
      <div class="w-full h-full flex items-center justify-center">
        <TriangleAlert class="text-error-500" size={32} />
      </div>
    </Avatar>
  {/snippet}
  {#snippet content()}
    <header>
      <p class="font-bold text-xl">Personal Access Token</p>
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
        <a target="_blank" href="https://github.com/settings/tokens" class="anchor"
          >Generate PAT here</a
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
          disabled={!hasPat}
          onclick={clearPat}>Clear</button
        >
      </form>
    </article>
  {/snippet}
</Popover>
