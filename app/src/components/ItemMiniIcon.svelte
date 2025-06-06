<script lang="ts">
  import { type WorkItemData } from "$lib/bindings/WorkItemData";
  import * as octicons from "@primer/octicons";

  type Props = {
    workItemData: WorkItemData;
  };

  let { workItemData }: Props = $props();

  let iconName: octicons.IconName = $derived.by(() => {
    if (workItemData.type === "draftIssue") {
      return "issue-draft";
    } else if (workItemData.type === "issue") {
      if (workItemData.state.loadState === "loaded") {
        if (workItemData.state.value === "OPEN") return "issue-opened";
        else if (workItemData.state.value === "CLOSED") return "issue-closed";
      }
    } else if (workItemData.type === "pullRequest") {
      if (workItemData.state.loadState === "loaded") {
        if (workItemData.state.value === "OPEN") return "git-pull-request";
        else if (workItemData.state.value === "CLOSED")
          return "git-pull-request-closed";
        else {
          return "git-merge";
        }
      }
    }

    return "question";
  });

  let icon = $derived(octicons[iconName]);
  let color = $derived.by(() => {
    switch (iconName) {
        case "issue-draft": return "text-gray-500";
        case "issue-opened": return "text-green-500";
        case "issue-closed": return "text-purple-500";
        case "git-pull-request": return "text-green-500";
        case "git-pull-request-closed": return "text-gray-500";
        case "git-merge": return "text-purple-500";
        default:
            return "text-gray-500";
    }
  });
</script>

{#if icon}
  <div class={`${color} inline`}>
    {@html icon.toSVG()}
  </div>
{/if}

<style>
  :global(.octicon) {
    display: inline-block;
    vertical-align: middle;
    fill: currentColor;
    overflow: visible;
  }
</style>
