<script lang="ts">
  import {
    NavItem,
    Button,
    Popover,
    Form,
    FormGroup,
    Input,
  } from "@sveltestrap/sveltestrap";
  import { invoke } from "@tauri-apps/api/core";

  let color = $state("secondary");
  let text = $state("...");
  let hasPat = $state(false);
  let isOpen = $state(false);

  async function update_status() {
    type Status = "NotSet" | "Set" | "Broken";

    const statusString = (await invoke("get_pat_status")) as String;
    const status = statusString as Status;

    switch (status) {
      case "NotSet":
        color = "danger";
        text = "Set PAT";
        hasPat = false;
        break;

      case "Set":
        color = "success";
        text = "PAT ok";
        hasPat = true;
        break;

      case "Broken":
        color = "danger";
        text = "PAT broken";
        hasPat = true;
        break;
    }
  }

  update_status().then(() => {
    if (!hasPat) isOpen = true;
  });

  const propsId = $props.id();
  const buttonId = `${propsId}-button`;

  let pat = $state("");

  async function setPat() {
    await invoke("set_pat", { pat: pat });
    await update_status();
    isOpen = false;
    pat = "";
  }

  async function clearPat() {
    await invoke("set_pat", { pat: "" });
    await update_status();
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
  <Button id={buttonId} size="sm" {color}>{text}</Button>
</NavItem>

<Popover
  bind:isOpen={() => isOpen, setIsOpen}
  hideOnOutsideClick
  container="body"
  target={buttonId}
  placement="auto"
  trigger="click"
  title="Personal Access Token"
  style="max-width:80em; min-width:40em"
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
