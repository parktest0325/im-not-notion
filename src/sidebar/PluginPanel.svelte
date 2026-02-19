<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Popup from "../component/Popup.svelte";
  import PluginInputPopup from "./PluginInputPopup.svelte";
  import { addToast } from "../stores";
  import type { PluginManifest, InputField } from "../types/setting";

  export let show: boolean;
  export let closePlugin: () => void;

  let plugins: PluginManifest[] = [];
  let isLoading = true;

  // Manual 플러그인 입력 팝업 상태
  let showInputPopup = false;
  let selectedPlugin: PluginManifest | null = null;
  let selectedInputFields: InputField[] = [];

  $: if (show) {
    loadPlugins();
  }

  async function loadPlugins() {
    isLoading = true;
    try {
      plugins = await invoke("list_plugins");
    } catch (error) {
      console.error("Failed to load plugins:", error);
      plugins = [];
      addToast("Failed to load plugins.");
    } finally {
      isLoading = false;
    }
  }

  function openManualInput(plugin: PluginManifest, inputFields: InputField[]) {
    selectedPlugin = plugin;
    selectedInputFields = inputFields;
    showInputPopup = true;
  }

  async function toggleCron(plugin: PluginManifest, schedule: string, label: string, enable: boolean) {
    try {
      if (enable) {
        await invoke("register_plugin_cron", {
          name: plugin.name,
          schedule,
          entry: plugin.entry,
        });
        addToast(`Cron "${label}" enabled.`, "success");
      } else {
        await invoke("unregister_plugin_cron", { name: plugin.name });
        addToast(`Cron "${label}" disabled.`, "success");
      }
    } catch (error) {
      console.error("Failed to toggle cron:", error);
      addToast("Failed to toggle cron.");
    }
  }

  async function handleRefreshTree() {
    try {
      await invoke("get_file_tree");
    } catch (e) {
      // 트리 새로고침 실패는 무시
    }
  }
</script>

<PluginInputPopup
  show={showInputPopup}
  plugin={selectedPlugin}
  inputFields={selectedInputFields}
  onClose={() => { showInputPopup = false; }}
  onRefreshTree={handleRefreshTree}
/>

<Popup {show} {isLoading} closePopup={closePlugin}>
  <h3 class="text-lg font-bold">Plugins</h3>

  {#if plugins.length === 0}
    <p class="text-sm opacity-50">No plugins found on server.</p>
    <p class="text-xs opacity-30">Place plugins in ~/.inn_plugins/ on the server.</p>
  {:else}
    <div class="space-y-3 max-h-96 overflow-y-auto">
      {#each plugins as plugin}
        <div class="p-3 rounded" style="background-color: var(--sidebar-bg-color);">
          <div class="flex justify-between items-start">
            <div>
              <span class="font-medium">{plugin.name}</span>
              <span class="text-xs opacity-50 ml-1">v{plugin.version}</span>
            </div>
          </div>
          <p class="text-xs opacity-60 mt-1">{plugin.description}</p>

          <div class="mt-2 space-y-1">
            {#each plugin.triggers as trigger}
              {#if trigger.type === "manual"}
                <button
                  class="text-xs px-2 py-1 rounded"
                  style="background-color: var(--button-active-bg-color);"
                  on:click={() => openManualInput(plugin, trigger.content.input)}
                >
                  {trigger.content.label}
                </button>
              {:else if trigger.type === "cron"}
                <div class="flex items-center justify-between text-xs">
                  <span class="opacity-60">{trigger.content.label} ({trigger.content.schedule})</span>
                  <div class="flex gap-1">
                    <button
                      class="px-2 py-0.5 rounded"
                      style="background-color: var(--button-active-bg-color);"
                      on:click={() => toggleCron(plugin, trigger.content.schedule, trigger.content.label, true)}
                    >On</button>
                    <button
                      class="px-2 py-0.5 rounded opacity-60"
                      on:click={() => toggleCron(plugin, trigger.content.schedule, trigger.content.label, false)}
                    >Off</button>
                  </div>
                </div>
              {:else if trigger.type === "hook"}
                <div class="text-xs opacity-60">
                  Hook: {trigger.content.event}
                </div>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <button class="w-full p-2 rounded mt-2 opacity-60" on:click={closePlugin}>
    Close
  </button>
</Popup>
