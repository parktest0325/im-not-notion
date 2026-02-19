<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Popup from "../component/Popup.svelte";
  import PluginInputPopup from "./PluginInputPopup.svelte";
  import { addToast, triggerPluginShortcut } from "../stores";
  import type { PluginInfo, InputField } from "../types/setting";
  import { registerAction, unregisterAction, pluginShortcutDefs, buildShortcutMap } from "../shortcut";
  import { onDestroy } from "svelte";

  export let show: boolean;
  export let closePlugin: () => void;

  let plugins: PluginInfo[] = [];
  let isLoading = true;
  let localPath = "";

  // Manual 플러그인 입력 팝업 상태
  let showInputPopup = false;
  let selectedPlugin: PluginInfo | null = null;
  let selectedInputFields: InputField[] = [];

  // 현재 등록된 플러그인 shortcut action ids (cleanup용)
  let registeredActionIds: string[] = [];

  $: if (show) {
    loadPlugins();
  }

  // triggerPluginShortcut store 감지 — 단축키로 플러그인 실행 요청
  $: if ($triggerPluginShortcut) {
    const req = $triggerPluginShortcut;
    triggerPluginShortcut.set(null);
    const plugin = plugins.find(p => p.manifest.name === req.pluginName);
    if (plugin) {
      if (!plugin.installed || !plugin.enabled) {
        addToast(`Plugin "${req.pluginName}" is not installed or enabled.`);
      } else {
        for (const trigger of plugin.manifest.triggers) {
          if (trigger.type === "manual" && trigger.content.label === req.triggerLabel) {
            openManualInput(plugin, trigger.content.input);
            break;
          }
        }
      }
    }
  }

  function registerPluginShortcuts() {
    // 기존 등록 해제
    for (const id of registeredActionIds) {
      unregisterAction(id);
    }
    registeredActionIds = [];

    const defs: Array<{ id: string; shortcut?: string; description: string }> = [];

    // 모든 플러그인의 manual trigger를 등록 (shortcut 유무 상관없이)
    // 실행 시점에 installed+enabled 체크
    for (const p of plugins) {
      for (const trigger of p.manifest.triggers) {
        if (trigger.type === "manual") {
          const actionId = `plugin:${p.manifest.name}:${trigger.content.label}`;
          const pluginName = p.manifest.name;
          const triggerLabel = trigger.content.label;

          defs.push({
            id: actionId,
            shortcut: trigger.content.shortcut,  // undefined if not set
            description: `${pluginName} - ${triggerLabel}`,
          });

          registerAction(actionId, () => {
            triggerPluginShortcut.set({ pluginName, triggerLabel });
          });
          registeredActionIds.push(actionId);
        }
      }
    }

    pluginShortcutDefs.set(defs);
    // Rebuild shortcut map (uses cached client overrides + new plugin defs)
    buildShortcutMap();
  }

  async function loadPlugins() {
    isLoading = true;
    try {
      plugins = await invoke("list_plugins", { localPath });
      registerPluginShortcuts();
    } catch (error) {
      console.error("Failed to load plugins:", error);
      plugins = [];
    } finally {
      isLoading = false;
    }
  }

  onDestroy(() => {
    for (const id of registeredActionIds) {
      unregisterAction(id);
    }
  });

  async function installPlugin(name: string) {
    try {
      await invoke("install_plugin", { localPath, name });
      addToast(`Installed: ${name}`, "success");
      await loadPlugins();
    } catch (error) {
      console.error("Install failed:", error);
      addToast(`Install failed: ${error}`);
    }
  }

  async function uninstallPlugin(name: string) {
    try {
      await invoke("uninstall_plugin", { name });
      addToast(`Uninstalled: ${name}`, "success");
      await loadPlugins();
    } catch (error) {
      console.error("Uninstall failed:", error);
      addToast(`Uninstall failed: ${error}`);
    }
  }

  async function toggleEnabled(name: string, enable: boolean) {
    try {
      if (enable) {
        await invoke("enable_plugin", { name });
        addToast(`Enabled: ${name}`, "success");
      } else {
        await invoke("disable_plugin", { name });
        addToast(`Disabled: ${name}`, "info");
      }
      await loadPlugins();
    } catch (error) {
      console.error("Toggle failed:", error);
      addToast(`Toggle failed: ${error}`);
    }
  }

  function openManualInput(plugin: PluginInfo, inputFields: InputField[]) {
    selectedPlugin = plugin;
    selectedInputFields = inputFields;
    showInputPopup = true;
  }

  async function toggleCron(name: string, schedule: string, entry: string, label: string, enable: boolean) {
    try {
      if (enable) {
        await invoke("register_plugin_cron", { name, schedule, entry });
        addToast(`Cron "${label}" enabled.`, "success");
      } else {
        await invoke("unregister_plugin_cron", { name });
        addToast(`Cron "${label}" disabled.`, "info");
      }
    } catch (error) {
      console.error("Cron toggle failed:", error);
      addToast("Cron toggle failed.");
    }
  }

  async function handleRefreshTree() {
    try {
      await invoke("get_file_tree");
    } catch (_) {}
  }
</script>

<PluginInputPopup
  show={showInputPopup}
  plugin={selectedPlugin?.manifest ?? null}
  inputFields={selectedInputFields}
  onClose={() => { showInputPopup = false; }}
  onRefreshTree={handleRefreshTree}
/>

<Popup {show} {isLoading} closePopup={closePlugin}>
  <h3 class="text-lg font-bold">Plugins</h3>

  <!-- Local path 입력 -->
  <div class="flex gap-2">
    <input
      type="text"
      class="flex-1 p-2 rounded text-sm"
      style="background-color: var(--input-bg-color); border: 1px solid var(--border-color);"
      bind:value={localPath}
      placeholder="/path/to/local/plugins"
      on:change={loadPlugins}
    />
    <button
      class="px-3 py-2 rounded text-sm"
      style="background-color: var(--button-active-bg-color);"
      on:click={loadPlugins}
    >Scan</button>
  </div>

  <!-- 플러그인 리스트 -->
  {#if plugins.length === 0}
    <p class="text-sm opacity-50">No plugins found.</p>
  {:else}
    <div class="space-y-3 max-h-80 overflow-y-auto">
      {#each plugins as p}
        <div class="p-3 rounded" style="background-color: var(--sidebar-bg-color);">
          <!-- 헤더: 이름 + 상태 뱃지 -->
          <div class="flex justify-between items-center">
            <div>
              <span class="font-medium">{p.manifest.name}</span>
              <span class="text-xs opacity-40 ml-1">v{p.manifest.version}</span>
            </div>
            <div class="flex gap-1">
              {#if p.installed}
                <span class="text-xs px-1.5 py-0.5 rounded bg-green-800 text-green-200">installed</span>
                {#if p.enabled}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-blue-800 text-blue-200">enabled</span>
                {:else}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-gray-700 text-gray-300">disabled</span>
                {/if}
              {:else}
                <span class="text-xs px-1.5 py-0.5 rounded bg-gray-700 text-gray-400">not installed</span>
              {/if}
            </div>
          </div>

          <p class="text-xs opacity-50 mt-1">{p.manifest.description}</p>

          <!-- 액션 버튼 -->
          <div class="flex gap-2 mt-2 flex-wrap">
            {#if p.installed}
              {#if p.enabled}
                <button class="text-xs px-2 py-1 rounded bg-yellow-800 text-yellow-200"
                  on:click={() => toggleEnabled(p.manifest.name, false)}>Disable</button>
              {:else}
                <button class="text-xs px-2 py-1 rounded bg-blue-800 text-blue-200"
                  on:click={() => toggleEnabled(p.manifest.name, true)}>Enable</button>
              {/if}
              <button class="text-xs px-2 py-1 rounded bg-red-900 text-red-200"
                on:click={() => uninstallPlugin(p.manifest.name)}>Uninstall</button>
            {:else}
              <button class="text-xs px-2 py-1 rounded"
                style="background-color: var(--button-active-bg-color);"
                on:click={() => installPlugin(p.manifest.name)}>Install</button>
            {/if}
          </div>

          <!-- Trigger별 컨트롤 (installed + enabled만) -->
          {#if p.installed && p.enabled}
            <div class="mt-2 space-y-1">
              {#each p.manifest.triggers as trigger}
                {#if trigger.type === "manual"}
                  <button
                    class="text-xs px-2 py-1 rounded"
                    style="background-color: var(--button-active-bg-color);"
                    on:click={() => openManualInput(p, trigger.content.input)}
                  >{trigger.content.label}</button>
                {:else if trigger.type === "cron"}
                  <div class="flex items-center justify-between text-xs">
                    <span class="opacity-60">{trigger.content.label} ({trigger.content.schedule})</span>
                    <div class="flex gap-1">
                      <button class="px-2 py-0.5 rounded"
                        style="background-color: var(--button-active-bg-color);"
                        on:click={() => toggleCron(p.manifest.name, trigger.content.schedule, p.manifest.entry, trigger.content.label, true)}
                      >On</button>
                      <button class="px-2 py-0.5 rounded opacity-60"
                        on:click={() => toggleCron(p.manifest.name, trigger.content.schedule, p.manifest.entry, trigger.content.label, false)}
                      >Off</button>
                    </div>
                  </div>
                {:else if trigger.type === "hook"}
                  <div class="text-xs opacity-50">Hook: {trigger.content.event}</div>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <button class="w-full p-2 rounded mt-2 opacity-60" on:click={closePlugin}>Close</button>
</Popup>
