<script lang="ts">
  import loader from "@monaco-editor/loader";
  import { editor } from "monaco-editor";

  let editorContainer: HTMLElement;
  let theEditor: editor.IStandaloneCodeEditor;

  type Props = {
    content: string;
    onSave: (content:string) => void;
    onCancel: () => void;
    schema?: any
  };

  let { content, onSave, onCancel, schema }: Props = $props();

  $effect(() => {
    loader.init().then((monaco) => {
        if (schema) {
            monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
                validate: true,                
                schemas: [
                    {
                        uri: "http://ghui/extra-data.json",
                        fileMatch: ["*"],
                        schema: schema
                    }
            ]});
        }
      theEditor = monaco.editor.create(editorContainer, {
        value: content,
        language: "json",
        theme: "vs-dark",
      });      
    });

    return () => {};
  });
</script>

<div class="w-full h-full flex flex-col gap-2">
  <div bind:this={editorContainer} class="flex-auto"></div>
  <div class="flex-initial pt-1 flex gap-1 place-content-end">
    <button class="btn preset-filled-primary-500" onclick={() => onSave(theEditor.getValue())}>Save</button>
    <button class="btn preset-filled-secondary-500" onclick={() => onCancel()}>Cancel</button>
  </div>
</div>
