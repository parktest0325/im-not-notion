<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../stores";
  import type { PluginManifest, InputField, PluginResult } from "../types/setting";

  export let show: boolean;
  export let plugin: PluginManifest | null = null;
  export let inputFields: InputField[] = [];
  export let onClose: () => void;
  export let onRefreshTree: () => void;
  export let onShowResult: (title: string, body: string, pages?: any[]) => void = () => {};
  export let onDownloadFiles: (items: any[]) => void = () => {};

  let values: Record<string, string | boolean> = {};
  let isExecuting = false;

  $: if (show && inputFields.length > 0) {
    values = {};
    for (const field of inputFields) {
      if (field.type === "boolean") {
        values[field.name] = field.default === "true";
      } else {
        values[field.name] = field.default ?? "";
      }
    }
  }

  async function executePlugin() {
    if (!plugin) return;
    isExecuting = true;
    try {
      // Read current values directly from DOM to avoid Svelte reactivity issues
      const formData: Record<string, string | boolean> = { trigger: "manual" };
      for (const field of inputFields) {
        if (field.type === "boolean") {
          const el = document.getElementById(field.name) as HTMLInputElement;
          formData[field.name] = el?.checked ?? false;
        } else {
          const el = document.getElementById(field.name) as HTMLInputElement;
          formData[field.name] = el?.value ?? values[field.name] ?? "";
        }
      }
      const inputJson = JSON.stringify(formData);
      const result: PluginResult = await invoke("run_plugin", {
        name: plugin.name,
        input: inputJson,
      });

      if (result.success) {
        addToast(result.message ?? "Plugin executed.", "success");
      } else {
        addToast(result.error ?? "Plugin failed.");
      }

      if (result.actions) {
        for (const action of result.actions) {
          if (action.type === "refresh_tree") {
            onRefreshTree();
          } else if (action.type === "toast" && action.content) {
            addToast(
              action.content.message,
              action.content.toast_type === "success" ? "success" : "error"
            );
          } else if (action.type === "show_result" && action.content) {
            onShowResult(action.content.title, action.content.body ?? "", action.content.pages);
          } else if (action.type === "download_files" && action.content) {
            onDownloadFiles(action.content.items);
          }
        }
      }

      onClose();
    } catch (error) {
      console.error("Plugin execution failed:", error);
      addToast("Plugin execution failed.");
    } finally {
      isExecuting = false;
    }
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="fixed inset-0 flex justify-center items-center p-4 input-overlay" on:click|self={onClose}>
    <div class="input-popup-content">
      {#if plugin}
        <h3 class="text-lg font-bold">{plugin.name}</h3>
        <p class="text-sm opacity-70">{plugin.description}</p>

        <div class="space-y-3">
          {#each inputFields as field}
            <div>
              {#if field.type === "boolean"}
                <label class="flex items-center gap-2 text-sm cursor-pointer" for={field.name}>
                  <input
                    id={field.name}
                    type="checkbox"
                    checked={values[field.name]}
                    on:change={(e) => { values[field.name] = e.currentTarget.checked; values = values; }}
                  />
                  {field.label}
                </label>
              {:else}
                <label class="block text-sm mb-1" for={field.name}>{field.label}</label>
                <input
                  id={field.name}
                  type={field.type === "password" ? "password" : "text"}
                  class="w-full p-2 rounded border"
                  style="background-color: var(--input-bg-color); border-color: var(--border-color);"
                  value={values[field.name]}
                  on:input={(e) => { values[field.name] = e.currentTarget.value; values = values; }}
                  placeholder={field.default ?? ""}
                />
              {/if}
            </div>
          {/each}
        </div>

        <button
          class="w-full p-2 rounded mt-2"
          style="background-color: var(--button-active-bg-color);"
          on:click={executePlugin}
          disabled={isExecuting}
        >
          {isExecuting ? "Executing..." : "Execute"}
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .input-overlay {
    background-color: var(--overlay-bg-color);
    z-index: 1100;
  }
  .input-popup-content {
    background-color: var(--popup-bg-color);
    color: var(--popup-text-color);
    padding: 1.5rem;
    border-radius: 0.5rem;
    box-shadow: var(--shadow-popup);
    width: 100%;
    max-width: 32rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
</style>
