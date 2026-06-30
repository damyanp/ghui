<script lang="ts">
  import { LoaderCircle, TriangleAlert } from "@lucide/svelte";
  import { Avatar } from "@skeletonlabs/skeleton-svelte";
  import Modal from "./Modal.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  type AuthStatus =
    | { type: "authenticated"; login: string; avatarUri: string }
    | { type: "needsProjectScope"; login: string; avatarUri: string }
    | { type: "checking" | "notAuthenticated" | "ghMissing" };

  let authState = $state<AuthStatus>({ type: "checking" });

  let isOpen = $state(false);

  function update_auth_status(status: AuthStatus) {
    authState = status;

    if (authState.type === "authenticated") isOpen = false;
    else if (authState.type != "checking") isOpen = true;
  }

  onMount(() => {
    let unregister: UnlistenFn | null = null;

    listen<AuthStatus>("auth-status", (e) => {
      update_auth_status(e.payload);
    }).then((u) => (unregister = u));
    invoke("check_auth_status");

    return () => {
      if (unregister) unregister();
    };
  });

  async function recheck() {
    authState = { type: "checking" };
    await invoke("check_auth_status");
  }

  let avatar_uri = $derived.by(() => {
    if (
      authState.type === "authenticated" ||
      authState.type === "needsProjectScope"
    )
      return authState.avatarUri;
    else return undefined;
  });

  let displayMode = $derived.by(() => {
    switch (authState.type) {
      case "notAuthenticated":
        return "Not Signed In";
      case "ghMissing":
        return "gh Not Found";
      case "needsProjectScope":
        return "Missing Project Access";
      case "checking":
        return "";
      default:
        return authState.type;
    }
  });

  let canClose = $derived.by(
    () =>
      authState.type == "authenticated" ||
      authState.type == "needsProjectScope",
  );
</script>

<button onclick={() => (isOpen = true)}>
  <!--
  For some reason the avatar doesn't get set correctly when sign-in changes.
  Inspecting avatar_uri shows that it does get set to 'undefined', but the
  fallback doesn't get shown.  Explicitly setting avatar_uri to a value and then
  to undefined does seem to get noticed.  Using #key makes this work, but it
  shouldn't be necessary.
  -->
  {#key avatar_uri}
    <Avatar class="size-12">
      <Avatar.Image
        src={avatar_uri}
        alt={authState.type === "authenticated" ||
        authState.type === "needsProjectScope"
          ? authState.login
          : "unknown"}
      />
      <Avatar.Fallback class="bg-error-500">
        <div class="w-full h-full flex items-center justify-center">
          {#if authState.type === "checking"}
            <LoaderCircle class="animate-spin" size={32} />
          {:else}
            <TriangleAlert class="text-error-500" size={32} />
          {/if}
        </div>
      </Avatar.Fallback>
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
        GitHub Sign-In
        {displayMode}
      </p>
    </header>
    <article>
      {#if authState.type === "ghMissing"}
        <p class="m-4">
          This app uses the <code>gh</code> CLI to talk to GitHub, but it
          couldn't be found. Install the GitHub CLI and make sure it's on your
          <code>PATH</code>.
        </p>
        <p class="m-4">
          <a
            target="_blank"
            href="https://cli.github.com/"
            class="anchor">Install the GitHub CLI</a
          >, then sign in by running <code>gh auth login</code> in a terminal.
        </p>
      {:else if authState.type === "needsProjectScope"}
        <p class="m-4">
          You're signed in as <strong>{authState.login}</strong>, but your
          <code>gh</code> token is missing the <code>project</code> scope needed
          to read and update GitHub Projects.
        </p>
        <p class="m-4">
          Run <code>gh auth refresh -s project</code> in a terminal to grant it,
          then re-check below.
        </p>
      {:else}
        <p class="m-4">
          This app uses the <code>gh</code> CLI to access GitHub on your behalf.
          Sign in once with the CLI to get started.
        </p>
        <p class="m-4">
          Run <code>gh auth login</code> in a terminal (granting access to
          projects &amp; repos), then re-check below.
        </p>
      {/if}

      <div class="w-full flex justify-end gap-2">
        <button class="btn preset-filled-primary-500" onclick={recheck}
          >Re-check</button
        >
      </div>
    </article>
  {/snippet}
</Modal>
