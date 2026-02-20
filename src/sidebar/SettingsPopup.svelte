<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import DynamicField from "../component/DynamicField.svelte";
  import HugoSetup from "./HugoSetup.svelte";
  import { createDefaultAppConfig, type AppConfig } from "../types/setting";
  import Popup from "../component/Popup.svelte";
  import { url, contentPath, hiddenPath, addToast } from "../stores";
  import { onMount } from "svelte";
  import { buildShortcutMap, getEffectiveShortcuts, eventToShortcutString, isRecordingShortcut, pluginShortcutDefs, registerAction, unregisterAction } from "../shortcut";
  import type { PluginInfo } from "../types/setting";

  export let show: boolean;
  export let closeSettings: () => void;

  const asFields = (obj: object): Record<string, string> =>
    obj as unknown as Record<string, string>;

  let config: AppConfig;
  let isLoading = true;
  let activeTab = "ssh";
  let isSetupRunning = false;

  // Shortcuts tab state
  let shortcutEntries: Array<{ id: string; description: string; shortcuts: string[] }> = [];
  let recordingAction: string | null = null;  // which action is being recorded
  let recordingIndex: number = -1;  // which shortcut index (-1 = adding new)

  function refreshShortcutEntries() {
    shortcutEntries = getEffectiveShortcuts(config?.shortcuts ?? {});
  }

  async function loadPluginShortcuts() {
    try {
      const plugins: PluginInfo[] = await invoke("list_plugins", { localPath: "" });
      const defs: Array<{ id: string; shortcut?: string; description: string }> = [];

      for (const p of plugins) {
        for (const trigger of p.manifest.triggers) {
          if (trigger.type === "manual") {
            defs.push({
              id: `plugin:${p.manifest.name}:${trigger.content.label}`,
              shortcut: trigger.content.shortcut,  // undefined if not set
              description: `${p.manifest.name} - ${trigger.content.label}`,
            });
          }
        }
      }

      pluginShortcutDefs.set(defs);
      buildShortcutMap();
      refreshShortcutEntries();
    } catch (_) {
      // SSH 미연결 등 — 플러그인 없이 빌트인만 표시
    }
  }

  function startRecording(actionId: string, index: number) {
    recordingAction = actionId;
    recordingIndex = index;
    isRecordingShortcut.set(true);
  }

  function cancelRecording() {
    recordingAction = null;
    recordingIndex = -1;
    isRecordingShortcut.set(false);
  }

  function handleShortcutRecord(event: KeyboardEvent) {
    if (!recordingAction) return;

    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      cancelRecording();
      return;
    }

    const shortcutStr = eventToShortcutString(event);
    if (!shortcutStr) return; // only modifier pressed

    if (!config.shortcuts) config.shortcuts = {};

    // Get current effective shortcuts for this action
    const entry = shortcutEntries.find(e => e.id === recordingAction);
    if (!entry) return;

    let keys = [...entry.shortcuts];
    if (recordingIndex >= 0 && recordingIndex < keys.length) {
      keys[recordingIndex] = shortcutStr; // replace existing
    } else {
      keys.push(shortcutStr); // add new
    }

    config.shortcuts[recordingAction!] = keys;
    refreshShortcutEntries();
    cancelRecording();
  }

  function removeShortcut(actionId: string, index: number) {
    if (!config.shortcuts) config.shortcuts = {};

    const entry = shortcutEntries.find(e => e.id === actionId);
    if (!entry) return;

    const keys = [...entry.shortcuts];
    keys.splice(index, 1);

    if (keys.length === 0) {
      // Empty array means explicitly no shortcuts
      config.shortcuts[actionId] = [];
    } else {
      config.shortcuts[actionId] = keys;
    }
    refreshShortcutEntries();
  }

  function resetShortcut(actionId: string) {
    if (!config.shortcuts) return;
    delete config.shortcuts[actionId];
    // Force Svelte reactivity
    config.shortcuts = { ...config.shortcuts };
    refreshShortcutEntries();
  }

  onMount(loadConfig);

  $: if (show) {
    // show가 true일 때만 설정 로드
    loadConfig();
  }

  $: if (activeTab === "shortcuts") {
    loadPluginShortcuts();
  }

  async function loadConfig() {
    isLoading = true; // 로딩 시작
    try {
      const loadedConfig : AppConfig = await invoke("load_config");
      config = {
        ...createDefaultAppConfig(),
        ...loadedConfig,
      };
      url.set(config.cms_config.hugo_config.url);
      contentPath.set(config.cms_config.hugo_config.content_path);
      hiddenPath.set(config.cms_config.hugo_config.hidden_path);
      buildShortcutMap(config.shortcuts ?? {}, []);
      refreshShortcutEntries();
    } catch (error) {
      console.error("Failed to load config:", error);
      config = createDefaultAppConfig();
      addToast("Failed to load settings.");
    } finally {
      isLoading = false; // 로딩 완료
    }
  }

  async function saveAndClose() {
    cancelRecording();
    try {
      await invoke("save_config", { config });
      await loadConfig(); // 저장 후 최신 상태 로드
      addToast("Settings saved.", "success");
    } catch (error) {
      console.error("Failed to save config:", error);
      addToast("Failed to save settings.");
    } finally {
      closeSettings();
    }
  }
</script>

<svelte:window on:keydown={handleShortcutRecord} />

<Popup {show} {isLoading} closePopup={() => { if (!isSetupRunning) closeSettings(); }}>
  <!-- 탭 버튼 -->
  <div class="flex space-x-4">
    <button
      class="tab-button"
      class:active={activeTab === "ssh"}
      on:click={() => (activeTab = "ssh")}
    >
      SSH Setting
    </button>
    <button
      class="tab-button"
      class:active={activeTab === "hugo"}
      on:click={() => (activeTab = "hugo")}
    >
      Hugo Setting
    </button>
    <button
      class="tab-button"
      class:active={activeTab === "shortcuts"}
      on:click={() => (activeTab = "shortcuts")}
    >
      Shortcuts
    </button>
  </div>

  <!-- 설정 입력 필드 -->
  {#if activeTab === "ssh"}
    <div class="space-y-4">
      {#each Object.keys(config.ssh_config) as key}
        <DynamicField config={asFields(config.ssh_config)} configKey={key} />
      {/each}
    </div>
  {:else if activeTab === "hugo"}
    <div class="space-y-4">
      <HugoSetup bind:config bind:isSetupRunning />
      {#each Object.keys(config.cms_config.hugo_config) as key}
        <DynamicField config={asFields(config.cms_config.hugo_config)} configKey={key} />
      {/each}
    </div>
  {:else if activeTab === "shortcuts"}
    <div class="space-y-3 max-h-80 overflow-y-auto">
      {#each shortcutEntries as entry}
        <div class="shortcut-row">
          <div class="shortcut-label">
            <span class="font-medium text-sm">{entry.description}</span>
            <span class="text-xs opacity-40 ml-1">({entry.id})</span>
          </div>
          <div class="shortcut-keys">
            {#each entry.shortcuts as key, i}
              {#if recordingAction === entry.id && recordingIndex === i}
                <span class="shortcut-badge recording">Press key...</span>
              {:else}
                <button
                  class="shortcut-badge"
                  on:click={() => startRecording(entry.id, i)}
                  title="Click to change"
                >{key}</button>
                <button
                  class="shortcut-remove"
                  on:click={() => removeShortcut(entry.id, i)}
                  title="Remove"
                >&times;</button>
              {/if}
            {/each}
            {#if recordingAction === entry.id && recordingIndex === -1}
              <span class="shortcut-badge recording">Press key...</span>
            {:else}
              <button
                class="shortcut-add"
                on:click={() => startRecording(entry.id, -1)}
                title="Add shortcut"
              >+</button>
            {/if}
            {#if config.shortcuts && entry.id in config.shortcuts}
              <button
                class="shortcut-reset"
                on:click={() => resetShortcut(entry.id)}
                title="Reset to default"
              >Reset</button>
            {/if}
          </div>
        </div>
      {/each}
      {#if recordingAction}
        <p class="text-xs opacity-50">Press a key combination to assign, or Escape to cancel.</p>
      {/if}
    </div>
  {/if}

  <!-- 공용 저장 버튼 -->
  <button class="save-button" on:click={saveAndClose} disabled={isSetupRunning}>
    Save and Exit
  </button>
</Popup>

<style>
  .tab-button {
    flex: 1;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    border: none;
    cursor: pointer;
    transition: background-color 0.25s;
  }

  .tab-button.active {
    background-color: var(--button-active-bg-color);
  }

  .save-button {
    width: 100%;
    padding: 0.75rem 1rem;
  }

  .shortcut-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    border-radius: 0.375rem;
    background-color: var(--sidebar-bg-color);
  }

  .shortcut-label {
    flex-shrink: 0;
    margin-right: 0.5rem;
  }

  .shortcut-keys {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-wrap: wrap;
  }

  .shortcut-badge {
    display: inline-block;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-family: monospace;
    background-color: var(--button-active-bg-color);
    cursor: pointer;
    border: 1px solid transparent;
    transition: border-color 0.15s;
  }

  .shortcut-badge:hover {
    border-color: var(--border-color);
  }

  .shortcut-badge.recording {
    border-color: var(--recording-border);
    background-color: var(--recording-bg);
    animation: pulse 1s infinite;
    cursor: default;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  .shortcut-remove {
    font-size: 0.75rem;
    opacity: 0.4;
    cursor: pointer;
    padding: 0 0.125rem;
    border: none;
    background: none;
  }

  .shortcut-remove:hover {
    opacity: 1;
    color: var(--shortcut-remove-hover);
  }

  .shortcut-add {
    font-size: 0.75rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    opacity: 0.4;
    cursor: pointer;
    border: 1px dashed var(--border-color);
    background: none;
  }

  .shortcut-add:hover {
    opacity: 1;
  }

  .shortcut-reset {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    opacity: 0.4;
    cursor: pointer;
    border: none;
    background: none;
    text-decoration: underline;
  }

  .shortcut-reset:hover {
    opacity: 1;
  }
</style>
