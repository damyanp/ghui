<script lang="ts">
  import { NavItem, Button } from "@sveltestrap/sveltestrap";
  import { invoke } from "@tauri-apps/api/core";

  let color = $state("secondary");
  let text = $state("...");

  async function update_status() {
    
    enum Status { NotSet };

    const statusString = (await invoke("get_pat_status")) as String;
    const status = statusString as keyof typeof Status;

    switch (status) {
      case "NotSet":
        color = "danger";
        text = "No PAT";
        break;
    }

  }

  update_status();

</script>

<NavItem>
  <Button size="sm" color={color}>{text}</Button>
</NavItem>
