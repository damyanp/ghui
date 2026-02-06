<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import WorkItemExtraDataEditor from "../../components/WorkItemExtraDataEditor.svelte";

  let text = $state("{}");
  let editorOpen = $state(false);
</script>

<h1>Editor Test</h1>

<div class="group">
  Blah blah blah <div
    class="inline group-hover:opacity-100 transition-opacity opacity-0"
  >
    <button onclick={() => (editorOpen = true)}>Edit</button>
    <WorkItemExtraDataEditor
      open={editorOpen}
      getInitialContent={() => text}
      onSave={(t) => { text = t; editorOpen = false; }}
      onCancel={() => (editorOpen = false)}
    >
      <h1>Blah blah blah</h1>
    </WorkItemExtraDataEditor>
  </div>
</div>

<pre>
  {text}
</pre>
