<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Popup from "../component/Popup.svelte";
  import PluginInputPopup from "./PluginInputPopup.svelte";
  import PluginResultPopup from "./PluginResultPopup.svelte";
  import PluginDownloadPopup from "./PluginDownloadPopup.svelte";
  import { addToast, triggerPluginShortcut } from "../stores";
  import type { PluginInfo, InputField, DownloadItem, AppConfig } from "../types/setting";
  import { registerAction, unregisterAction, pluginShortcutDefs, buildShortcutMap } from "../shortcut";
  import { onMount, onDestroy } from "svelte";

  export let show: boolean;
  export let closePlugin: () => void;

  let plugins: PluginInfo[] = [];
  let isLoading = true;
  let localPath = "";
  let searchQuery = "";

  $: filteredPlugins = searchQuery.trim()
    ? plugins.filter(p => p.manifest.name.toLowerCase().includes(searchQuery.trim().toLowerCase()))
    : plugins;

  // Manual 플러그인 입력 팝업 상태
  let showInputPopup = false;
  let selectedPlugin: PluginInfo | null = null;
  let selectedInputFields: InputField[] = [];

  // 결과 팝업 상태
  let showResultPopup = false;
  let resultTitle = "";
  let resultBody = "";
  let resultPages: any[] = [];

  // 다운로드 팝업 상태
  let showDownloadPopup = false;
  let downloadItems: DownloadItem[] = [];

  // Cron 등록 상태: "pluginName:label" → true
  let cronEnabled: Set<string> = new Set();

  function cronKey(name: string, label: string) { return `${name}:${label}`; }

  // 현재 등록된 플러그인 shortcut action ids (cleanup용)
  let registeredActionIds: string[] = [];

  // 마운트 시 저장된 경로 복원 → 플러그인 로드 → 숏컷 등록
  onMount(async () => {
    try {
      const config: AppConfig = await invoke("load_config");
      if (config.plugin_local_path) {
        localPath = config.plugin_local_path;
      }
    } catch (_) {}
    loadPlugins();
  });

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

  function saveLocalPath() {
    invoke("save_plugin_local_path", { path: localPath }).catch(() => {});
  }

  async function loadPlugins() {
    isLoading = true;
    try {
      const raw: PluginInfo[] = await invoke("list_plugins", { localPath });
      plugins = raw.sort((a, b) => {
        // server(installed) first, local-only second, then alphabetical
        const rank = (p: PluginInfo) => p.installed ? 0 : 1;
        const diff = rank(a) - rank(b);
        if (diff !== 0) return diff;
        return a.manifest.name.localeCompare(b.manifest.name);
      });
      registerPluginShortcuts();

      // Cron 등록 상태 조회 (installed 플러그인이 있을 때만)
      if (plugins.some(p => p.installed && p.enabled)) {
        try {
          const crons: string[] = await invoke("list_registered_crons");
          cronEnabled = new Set(crons);
        } catch (_) {
          cronEnabled = new Set();
        }
      } else {
        cronEnabled = new Set();
      }
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
        await invoke("register_plugin_cron", { name, schedule, entry, label });
        cronEnabled.add(cronKey(name, label));
        cronEnabled = cronEnabled;
        addToast(`Cron "${label}" enabled.`, "success");
      } else {
        await invoke("unregister_plugin_cron", { name, label });
        cronEnabled.delete(cronKey(name, label));
        cronEnabled = cronEnabled;
        addToast(`Cron "${label}" disabled.`, "info");
      }
    } catch (error) {
      console.error("Cron toggle failed:", error);
      addToast(`Cron failed: ${error}`);
    }
  }

  async function pullPlugin(name: string) {
    try {
      await invoke("pull_plugin", { localPath, name });
      addToast(`Pulled: ${name}`, "success");
      await loadPlugins();
    } catch (error) {
      addToast(`Pull failed: ${error}`);
    }
  }

  async function openInEditor(name: string) {
    try {
      await invoke("open_plugin_in_editor", { localPath, name });
    } catch (error) {
      addToast(`Failed to open editor: ${error}`);
    }
  }

  async function handleRefreshTree() {
    try {
      await invoke("get_file_tree");
    } catch (_) {}
  }

  function handleShowResult(title: string, body: string, pages?: any[]) {
    resultTitle = title;
    resultBody = body;
    resultPages = pages ?? [];
    showResultPopup = true;
  }

  function handleDownloadFiles(items: DownloadItem[]) {
    downloadItems = items;
    showDownloadPopup = true;
  }
</script>

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
      on:change={() => { saveLocalPath(); loadPlugins(); }}
    />
    <button
      class="px-3 py-2 rounded text-sm"
      style="background-color: var(--button-active-bg-color);"
      on:click={() => { saveLocalPath(); loadPlugins(); }}
    >Scan</button>
  </div>

  <!-- 플러그인 검색 -->
  {#if plugins.length > 0}
    <input
      type="text"
      class="w-full p-2 rounded text-sm"
      style="background-color: var(--input-bg-color); border: 1px solid var(--border-color);"
      bind:value={searchQuery}
      placeholder="Search plugins..."
    />
  {/if}

  <!-- 플러그인 리스트 -->
  {#if plugins.length === 0}
    <p class="text-sm opacity-50">No plugins found.</p>
  {:else if filteredPlugins.length === 0}
    <p class="text-sm opacity-50">No matching plugins.</p>
  {:else}
    <div class="space-y-3 max-h-80 overflow-y-auto">
      {#each filteredPlugins as p}
        <div class="plugin-card" class:plugin-active={p.installed && p.enabled}>
          <!-- 헤더: 이름 + 상태 뱃지 -->
          <div class="flex justify-between items-center">
            <div>
              <span class="font-medium">{p.manifest.name}</span>
              <span class="text-xs opacity-40 ml-1">v{p.manifest.version}</span>
            </div>
            <div class="flex gap-1">
              {#if p.local}
                <span class="text-xs px-1.5 py-0.5 rounded" style="background-color: var(--badge-local-bg); color: var(--badge-local-text);">local</span>
              {/if}
              {#if p.installed}
                <span class="text-xs px-1.5 py-0.5 rounded" style="background-color: var(--badge-server-bg); color: var(--badge-server-text);">server</span>
              {/if}
            </div>
          </div>

          <p class="text-xs opacity-50 mt-1">{p.manifest.description}</p>

          <!-- 액션 버튼 -->
          <div class="flex justify-between items-center mt-2">
            <div class="flex gap-1.5 items-center flex-wrap">
              {#if p.local && !p.installed}
                <button class="text-xs px-2 py-1 rounded"
                  style="background-color: var(--button-active-bg-color);"
                  on:click={() => installPlugin(p.manifest.name)}>Install</button>
              {/if}
              {#if p.installed}
                <button
                  class="toggle-btn"
                  class:toggle-on={p.enabled}
                  title={p.enabled ? "Disable" : "Enable"}
                  on:click={() => toggleEnabled(p.manifest.name, !p.enabled)}
                ><span class="toggle-dot"></span></button>
                <button class="text-xs px-2 py-1 rounded"
                  style="background-color: var(--btn-danger-bg); color: var(--btn-danger-text);"
                  on:click={() => uninstallPlugin(p.manifest.name)}>Uninstall</button>
              {/if}
            </div>
            <div class="flex gap-1 items-center">
              {#if p.local && p.installed && !p.synced}
                <span class="sync-warn" title="Out of sync">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--sync-warn-color)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
                  </svg>
                </span>
              {/if}
              {#if p.local && p.installed && localPath}
                <button class="icon-btn" title="Upload to server"
                  on:click={() => installPlugin(p.manifest.name)}
                ><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>
                </svg></button>
              {/if}
              {#if p.installed && localPath}
                <button class="icon-btn" title="Download from server"
                  on:click={() => pullPlugin(p.manifest.name)}
                ><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
                </svg></button>
              {/if}
              {#if p.local && localPath}
                <button class="icon-btn" title="Open in VS Code"
                  on:click={() => openInEditor(p.manifest.name)}
                ><svg width="14" height="14" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <mask id="m-{p.manifest.name}" mask-type="alpha" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="100">
                    <path fill-rule="evenodd" clip-rule="evenodd" d="M70.9119 99.3171C72.4869 99.9307 74.2828 99.8914 75.8725 99.1264L96.4608 89.2197C98.6242 88.1787 100 85.9892 100 83.5872V16.4133C100 14.0113 98.6242 11.8218 96.4608 10.7808L75.8725 0.874054C73.7862 -0.130386 71.3446 0.11025 69.5135 1.44315L29.2715 33.0719L12.1227 19.8564C10.5077 18.5823 8.21637 18.6902 6.72622 20.1132L1.29044 25.3263C-0.429983 26.9674 -0.430892 29.7481 1.28853 31.3904L16.1318 45.2548L1.28853 59.1192C-0.430892 60.7616 -0.429983 63.5765 1.29044 65.2176L6.72622 70.4307C8.21637 71.8537 10.5077 71.9273 12.1227 70.6532L29.2715 57.4377L69.5135 89.0665C69.9254 89.4142 70.4013 89.6771 70.9119 89.8405V99.3171ZM75.0152 27.2989L45.1091 50.2548L75.0152 73.2107V27.2989Z" fill="white"/>
                  </mask>
                  <g mask="url(#m-{p.manifest.name})">
                    <path d="M96.4614 10.7962L75.8569 0.875542C73.4719 -0.272773 70.6217 0.211611 68.75 2.08333L1.29688 59.1684C-0.390625 60.7893 -0.390625 63.5765 1.29688 65.2176L6.71875 70.4307C8.1875 71.8537 10.5 71.9273 12.0938 70.6532L93.75 8.33333C96.875 5.83333 100 7.5 100 11.6667V11.4133C100 9.01128 98.6243 6.82178 96.4614 5.78076L96.4614 10.7962Z" fill="#0065A9"/>
                    <g filter="url(#f1-{p.manifest.name})">
                      <path d="M96.4614 89.2038L75.8569 99.1245C73.4719 100.273 70.6217 99.7884 68.75 97.9167L1.29688 40.8316C-0.390625 39.2107 -0.390625 36.4235 1.29688 34.7824L6.71875 29.5693C8.1875 28.1463 10.5 28.0727 12.0938 29.3468L93.75 91.6667C96.875 94.1667 100 92.5 100 88.3333V88.5867C100 90.9887 98.6243 93.1782 96.4614 94.2192L96.4614 89.2038Z" fill="#007ACC"/>
                    </g>
                    <g filter="url(#f2-{p.manifest.name})">
                      <path d="M75.8578 99.1263C73.4721 100.274 70.6219 99.7885 68.75 97.9166C71.875 100.417 75 98.75 75 94.5833V5.41663C75 1.24996 71.875 -0.416707 68.75 2.08329C70.6219 0.211576 73.4721 -0.273394 75.8578 0.874641L96.4566 10.7813C98.6195 11.8224 100 14.0119 100 16.4139V83.5861C100 85.9881 98.6195 88.1776 96.4566 89.2187L75.8578 99.1263Z" fill="#1F9CF0"/>
                    </g>
                    <rect opacity="0.25" fill="url(#p0-{p.manifest.name})" width="100" height="100"/>
                  </g>
                  <defs>
                    <filter id="f1-{p.manifest.name}" x="-8.4" y="15.1" width="116.8" height="92.8" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
                      <feFlood flood-opacity="0" result="bg"/><feBlend in="SourceGraphic" in2="bg"/><feGaussianBlur stdDeviation="4.17"/>
                    </filter>
                    <filter id="f2-{p.manifest.name}" x="60.4" y="-8.6" width="48" height="117.1" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
                      <feFlood flood-opacity="0" result="bg"/><feBlend in="SourceGraphic" in2="bg"/><feGaussianBlur stdDeviation="4.17"/>
                    </filter>
                    <linearGradient id="p0-{p.manifest.name}" x1="50" y1="0" x2="50" y2="100" gradientUnits="userSpaceOnUse">
                      <stop stop-color="white"/><stop offset="1" stop-opacity="0"/>
                    </linearGradient>
                  </defs>
                </svg></button>
              {/if}
            </div>
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
                    <button
                      class="toggle-btn"
                      class:toggle-on={cronEnabled.has(cronKey(p.manifest.name, trigger.content.label))}
                      title={cronEnabled.has(cronKey(p.manifest.name, trigger.content.label)) ? "Disable cron" : "Enable cron"}
                      on:click={() => toggleCron(
                        p.manifest.name,
                        trigger.content.schedule,
                        p.manifest.entry,
                        trigger.content.label,
                        !cronEnabled.has(cronKey(p.manifest.name, trigger.content.label))
                      )}
                    ><span class="toggle-dot"></span></button>
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

<PluginInputPopup
  show={showInputPopup}
  plugin={selectedPlugin?.manifest ?? null}
  inputFields={selectedInputFields}
  onClose={() => { showInputPopup = false; }}
  onRefreshTree={handleRefreshTree}
  onShowResult={handleShowResult}
  onDownloadFiles={handleDownloadFiles}
/>

<PluginResultPopup
  show={showResultPopup}
  title={resultTitle}
  body={resultBody}
  pages={resultPages}
  onClose={() => { showResultPopup = false; }}
/>

<PluginDownloadPopup
  show={showDownloadPopup}
  items={downloadItems}
  onClose={() => { showDownloadPopup = false; }}
/>

<style>
  .plugin-card {
    padding: 0.75rem;
    border-radius: 0.375rem;
    background-color: var(--sidebar-bg-color);
    border: 1px solid var(--border-color);
    transition: border-color 0.2s;
  }
  .plugin-card.plugin-active {
    border-color: var(--active-border-color);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 0.25rem;
    border: 1px solid var(--border-color);
    background: none;
    cursor: pointer;
    opacity: 0.7;
  }
  .icon-btn:hover {
    opacity: 1;
    background-color: var(--button-active-bg-color);
  }

  .toggle-btn {
    position: relative;
    width: 32px;
    height: 18px;
    border-radius: 9px;
    border: none;
    background-color: var(--toggle-off-bg);
    cursor: pointer;
    padding: 0;
    transition: background-color 0.2s;
  }
  .toggle-btn.toggle-on {
    background-color: var(--toggle-on-bg);
  }
  .toggle-dot {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s;
  }
  .toggle-btn.toggle-on .toggle-dot {
    transform: translateX(14px);
  }

  .sync-warn {
    display: flex;
    align-items: center;
  }
</style>
