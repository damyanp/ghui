<script lang="ts">
  import {
    NavItem,
    Button,
    Popover,
    Form,
    FormGroup,
    Input,
    Tooltip,
  } from "@sveltestrap/sveltestrap";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";

  let color = $state("secondary");
  let text = $state("...");
  let hasPat = $state(false);
  let isOpen = $state(false);
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

<NavItem>
  <Button id={buttonId} size="sm" {color}>
    {#if avatar}
      <img src={avatar} alt={text} style="width: 32px; height: 32px; aspect-ratio: auto 32 / 32;" />
      <Tooltip animation target={buttonId}>{text}</Tooltip>
    {:else}
      {text}
    {/if}
  </Button>
</NavItem>

<Popover
  bind:isOpen={() => isOpen, setIsOpen}
  hideOnOutsideClick
  container="body"
  target={buttonId}
  placement="auto"
  trigger="click"
  title="Personal Access Token"
  style="max-width:30em; min-width:40em"
>
  <p>
    This app needs access to github. Although we'd do a proper github login
    flow, the llvm project requires github apps to be installed, which isn't
    great for experiments. Instead we use a PAT. This means that as far as
    github is concerned, this app is you. When you generate the PAT you can
    control what it has access to.
  </p>

  <p>
    <a target="_blank" href="https://github.com/settings/tokens"
      >Generate PAT here</a
    >. Tokens (classic) with access to projects seem to work well.
  </p>

  <Form>
    <FormGroup floating label="Paste PAT here">
      <Input type="password" bind:value={pat} />
    </FormGroup>

    <Button color="primary" disabled={pat.length == 0} on:click={setPat}
      >Set</Button
    >
    <Button color="danger" disabled={!hasPat} on:click={clearPat}>Clear</Button>
  </Form>
</Popover>
