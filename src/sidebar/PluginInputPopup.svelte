<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Popup from "../component/Popup.svelte";
  import { addToast } from "../stores";
  import type { PluginManifest, InputField, PluginResult } from "../types/setting";

  export let show: boolean;
  export let plugin: PluginManifest | null = null;
  export let inputFields: InputField[] = [];
  export let onClose: () => void;
  export let onRefreshTree: () => void;
  export let onShowResult: (title: string, body: string) => void = () => {};

  let values: Record<string, string> = {};
  let isExecuting = false;

  $: if (show && inputFields.length > 0) {
    values = {};
    for (const field of inputFields) {
      values[field.name] = field.default ?? "";
    }
  }

  async function executePlugin() {
    if (!plugin) return;
    isExecuting = true;
    try {
      const inputJson = JSON.stringify({
        trigger: "manual",
        input: values,
      });
      const result: PluginResult = await invoke("run_plugin", {
        name: plugin.name,
        input: inputJson,
      });

      if (result.success) {
        addToast(result.message ?? "Plugin executed.", "success");
      } else {
        addToast(result.error ?? "Plugin failed.");
      }

      // actions 처리
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
            onShowResult(action.content.title, action.content.body);
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

<Popup {show} closePopup={onClose}>
  {#if plugin}
    <h3 class="text-lg font-bold">{plugin.name}</h3>
    <p class="text-sm opacity-70">{plugin.description}</p>

    <div class="space-y-3">
      {#each inputFields as field}
        <div>
          <label class="block text-sm mb-1" for={field.name}>{field.label}</label>
          <input
            id={field.name}
            type="text"
            class="w-full p-2 rounded border"
            style="background-color: var(--input-bg-color); border-color: var(--border-color);"
            bind:value={values[field.name]}
            placeholder={field.default ?? ""}
          />
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
</Popup>
