<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import JsonEditor from "./JsonEditor.svelte";
  import type { Snippet } from "svelte";

  type Props = {
    open: boolean;
    getInitialContent: () => string;
    onSave: (content: string) => void;
    onCancel: () => void;
    children?: Snippet;
  };

  let props: Props = $props();

  const stateEnum = {
    enum: [
      "completed",
      "onTrack",
      "atRisk",
      "offTrack",
      "notStarted",
      "noDates",
    ],
  };

  const barSchema = {
    type: "object",
    properties: {
      state: stateEnum,
      label: { type: "string" },
      start: { type: "string" },
      end: { type: "string" },
    },
  };

  const schema = {
    type: "object",
    properties: {
      bars: {
        type: "array",
        items: {
          ...barSchema,
          required: ["label", "state", "start", "end"],
        },
      },
      split: {
        type: "array",
        items: {
          ...barSchema,
          required: ["state", "label"],
        },
      },
      start: { type: "string" },
      estimate: { type: "number" },
      burnDown: stateEnum,
    },
  };
</script>

<Modal
  open={props.open}
  contentBase="bg-surface-100-900 p-4 space-y-4 shadow-xl w-[480px] h-screen flex flex-col"
  positionerJustify="justify-start"
  positionerAlign=""
  positionerPadding=""
  transitionsPositionerIn={{ x: -480, duration: 200 }}
  transitionsPositionerOut={{ x: -480, duration: 200 }}
  closeOnEscape={false}
  closeOnInteractOutside={false}
  modal={false}
>
  {#snippet content()}
    {@render props.children?.()}
    <JsonEditor
      {schema}
      onSave={(c) => {
        props.onSave(c);
      }}
      onCancel={props.onCancel}
      content={props.getInitialContent()}
    />
  {/snippet}
</Modal>
