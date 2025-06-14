<script lang="ts">
  import { Modal } from "@skeletonlabs/skeleton-svelte";
  import JsonEditor from "../../components/WorkItemExtraDataEditor.svelte";
  import WorkItemExtraDataEditor from "../../components/WorkItemExtraDataEditor.svelte";

  let text = $state("{}");
</script>

<h1>Editor Test</h1>

<div class="group">
  Blah blah blah <div
    class="inline group-hover:opacity-100 transition-opacity opacity-0"
  >
    <WorkItemExtraDataEditor content={text} onSave={(t) => (text = t)}>
      <h1>Blah blah blah</h1>
    </WorkItemExtraDataEditor>
  </div>
</div>

<pre>
  {text}
</pre>
