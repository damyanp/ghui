<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import JsonEditor from "./JsonEditor.svelte";
  import { Pencil } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  type Props = {
    content: string;
    onSave: (content: string) => void;
    children?: Snippet;
  };

  let props: Props = $props();

  let editorOpen = $state(false);

  const schema = {
    type: "object",
    properties: {
      bars: {
        type: "array",
        items: {
          type: "object",
          properties: {
            state: {
              enum: [
                "completed",
                "onTrack",
                "atRisk",
                "offTrack",
                "notStarted",
                "noDates",
              ],
            },
            label: { type: "string" },
            start: { type: "string" },
            end: { type: "string" },
          },
          required: ["state", "start", "end"]
        },
      },
    },
  };;
</script>

<Modal
  open={editorOpen}
  onOpenChange={(e) => (editorOpen = e.open)}
  triggerBase="btn btn-sm"
  contentBase="bg-surface-100-900 p-4 space-y-4 shadow-xl w-[480px] h-screen flex flex-col"
  positionerJustify="justify-start"
  positionerAlign=""
  positionerPadding=""
  transitionsPositionerIn={{ x: -480, duration: 200 }}
  transitionsPositionerOut={{ x: -480, duration: 200 }}
>
  {#snippet trigger()}
    <Pencil size={10} />
  {/snippet}
  {#snippet content()}
    {@render props.children?.()}
    <JsonEditor
      {schema}
      onSave={(c) => {
        props.onSave(c);
        editorOpen = false;
      }}
      onCancel={() => {
        editorOpen = false;
      }}
      content={props.content}
    />
  {/snippet}
</Modal>
