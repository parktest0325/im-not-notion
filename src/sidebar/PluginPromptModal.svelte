<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { PromptKind, type PluginPrompt } from "../types/generated";

  export let prompt: PluginPrompt | null;

  const dispatch = createEventDispatcher<{
    respond: { id: string; value: any };
    cancel: { id: string };
  }>();

  let selectedValues: Set<string> = new Set();
  let inputValue: string = "";

  $: if (prompt) {
    selectedValues = new Set();
    inputValue = prompt.default_value ?? "";
  }

  function toggleSelect(value: string) {
    if (prompt?.multiple) {
      if (selectedValues.has(value)) selectedValues.delete(value);
      else selectedValues.add(value);
      selectedValues = selectedValues;
    } else {
      selectedValues = new Set([value]);
    }
  }

  function submit() {
    if (!prompt) return;
    let value: any;
    if (prompt.kind === PromptKind.Confirm) {
      value = true;
    } else if (prompt.kind === PromptKind.Select) {
      if (prompt.multiple) {
        value = Array.from(selectedValues);
      } else {
        value = Array.from(selectedValues)[0] ?? null;
      }
    } else {
      value = inputValue;
    }
    dispatch("respond", { id: prompt.id, value });
  }

  function cancel() {
    if (!prompt) return;
    if (prompt.kind === PromptKind.Confirm) {
      dispatch("respond", { id: prompt.id, value: false });
    } else {
      dispatch("cancel", { id: prompt.id });
    }
  }
</script>

{#if prompt}
<div class="fixed inset-0 bg-black/70 flex items-center justify-center" style="z-index: 1250;" role="dialog">
  <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-[500px] max-h-[80vh] flex flex-col p-5">
    <h3 class="text-sm font-semibold mb-1">{prompt.title}</h3>
    <p class="text-xs text-zinc-400 mb-1">{prompt.plugin}</p>
    {#if prompt.message}
      <p class="text-xs text-zinc-300 mb-3 whitespace-pre-line">{prompt.message}</p>
    {/if}

    <div class="flex-1 overflow-auto mb-4">
      {#if prompt.kind === PromptKind.Confirm}
        <!-- nothing extra; confirm = ok/cancel buttons -->
      {:else if prompt.kind === PromptKind.Select}
        <div class="space-y-1">
          {#each prompt.items as item}
            <label class="flex items-start gap-2 px-2 py-1.5 rounded hover:bg-zinc-800 cursor-pointer text-xs">
              <input
                type={prompt.multiple ? "checkbox" : "radio"}
                checked={selectedValues.has(item.value)}
                on:change={() => toggleSelect(item.value)}
                class="mt-0.5"
              />
              <div class="flex-1 min-w-0">
                <div class="truncate">{item.label}</div>
                {#if item.description}
                  <div class="text-[10px] text-zinc-500 truncate">{item.description}</div>
                {/if}
              </div>
            </label>
          {/each}
        </div>
      {:else if prompt.kind === PromptKind.Input}
        <input
          type="text"
          class="w-full bg-zinc-800 border border-zinc-700 rounded px-2 py-1 text-xs"
          bind:value={inputValue}
          on:keydown={(e) => e.key === "Enter" && submit()}
        />
      {/if}
    </div>

    <div class="flex gap-2 justify-end">
      <button
        class="px-3 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-xs"
        on:click={cancel}
      >
        Cancel
      </button>
      <button
        class="px-3 py-1 bg-blue-700 hover:bg-blue-600 rounded text-xs"
        on:click={submit}
        disabled={prompt.kind === PromptKind.Select && selectedValues.size === 0}
      >
        {prompt.kind === PromptKind.Confirm ? "OK" : "Submit"}
      </button>
    </div>
  </div>
</div>
{/if}
