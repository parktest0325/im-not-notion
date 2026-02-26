<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import DynamicField from "../component/DynamicField.svelte";
  import HugoSetup from "./HugoSetup.svelte";
  import { createDefaultAppConfig, createDefaultSshConfig, createDefaultServerEntry, type AppConfig, type ServerEntry } from "../types/setting";
  import Popup from "../component/Popup.svelte";
  import { url, contentPaths, hiddenPath, addToast, activeServerName } from "../stores";
  import { onMount } from "svelte";
  import { buildShortcutMap, getEffectiveShortcuts, eventToShortcutString, isRecordingShortcut, pluginShortcutDefs } from "../shortcut";
  import type { PluginInfo } from "../types/setting";

  export let show: boolean;
  export let closeSettings: () => void;
  export let onServerSwitch: () => void;

  const asFields = (obj: object): Record<string, string> =>
    obj as unknown as Record<string, string>;

  let config: AppConfig;
  let isLoading = true;

  // ── View state ──
  // "list" = 서버 목록, "edit" = 서버 편집
  let view: "list" | "edit" = "list";
  let editingServer: ServerEntry | null = null;
  let isNewServer = false;
  let editTab: "ssh" | "hugo" | "shortcuts" = "ssh";
  let isSetupRunning = false;
  let isSwitching = false;
  let isConnected = true;
  let deletingServerId: string | null = null;

  // Shortcuts state
  let shortcutEntries: Array<{ id: string; description: string; shortcuts: string[] }> = [];
  let recordingAction: string | null = null;
  let recordingIndex: number = -1;

  // ── Config loading ──

  onMount(loadConfig);

  $: if (show) {
    loadConfig();
    view = "list";
    editingServer = null;
  }

  async function loadConfig() {
    isLoading = true;
    try {
      const loadedConfig: AppConfig = await invoke("load_config");
      config = { ...createDefaultAppConfig(), ...loadedConfig };
      if (!config.servers) config.servers = [];
      url.set(config.cms_config.hugo_config.url);
      contentPaths.set(config.cms_config.hugo_config.content_paths);
      hiddenPath.set(config.cms_config.hugo_config.hidden_path);
      const active = config.servers.find(s => s.id === config.active_server);
      activeServerName.set(active?.name ?? "");
      buildShortcutMap(config.shortcuts ?? {}, []);
      refreshShortcutEntries();
      // load_config 내부에서 이미 SSH 연결을 시도하므로,
      // 설정이 성공적으로 로드되었으면 연결된 것으로 판단 (별도 check_connection 불필요)
      isConnected = !!config.cms_config.hugo_config.base_path;
    } catch (error) {
      console.error("Failed to load config:", error);
      config = createDefaultAppConfig();
      isConnected = false;
      addToast("Failed to load settings.");
    } finally {
      isLoading = false;
    }
  }

  // ── Server switching ──

  async function selectServer(id: string) {
    if (config.active_server === id) return;
    await connectServer(id);
  }

  async function reconnectServer() {
    await connectServer(config.active_server);
  }

  async function connectServer(id: string) {
    isSwitching = true;
    try {
      const newConfig: AppConfig = await invoke("switch_server", {
        servers: config.servers,
        serverId: id,
      });
      config = {
        ...config,
        active_server: id,
        servers: newConfig.servers ?? config.servers,
        cms_config: newConfig.cms_config,
        shortcuts: newConfig.shortcuts,
      };
      url.set(config.cms_config.hugo_config.url);
      contentPaths.set(config.cms_config.hugo_config.content_paths);
      hiddenPath.set(config.cms_config.hugo_config.hidden_path);
      const active = config.servers?.find(s => s.id === id);
      activeServerName.set(active?.name ?? "");
      isConnected = true;
      refreshShortcutEntries();
      addToast("Server connected.", "success");
      onServerSwitch();
    } catch (error) {
      console.error("Failed to connect server:", error);
      isConnected = false;
      addToast(`Failed to connect: ${error}`);
    } finally {
      isSwitching = false;
    }
  }

  // ── Server CRUD ──

  function openAddServer() {
    editingServer = createDefaultServerEntry();
    editingServer.id = crypto.randomUUID().replace(/-/g, "").slice(0, 16);
    isNewServer = true;
    editTab = "ssh";
    view = "edit";
  }

  function openEditServer(server: ServerEntry) {
    editingServer = { ...server, ssh_config: { ...server.ssh_config } };
    isNewServer = false;
    editTab = "ssh";
    view = "edit";
  }

  function backToList() {
    cancelRecording();
    editingServer = null;
    view = "list";
  }

  /// SSH 탭에서 Hugo/Shortcuts 탭으로 전환 시:
  /// 현재 SSH 설정으로 서버 목록 업데이트 → 해당 서버로 전환(연결) → 서버 설정 로드
  async function switchEditTab(tab: "ssh" | "hugo" | "shortcuts") {
    if (tab === editTab) return;

    // SSH → 다른 탭으로 넘어갈 때: SSH 설정 저장 + 연결 + 서버 설정 로드
    if (editTab === "ssh" && tab !== "ssh" && editingServer) {
      isSwitching = true;
      try {
        // 서버 목록에 현재 편집 중인 서버 반영
        if (!config.servers) config.servers = [];
        if (isNewServer && !config.servers.find(s => s.id === editingServer!.id)) {
          config.servers = [...config.servers, editingServer];
          isNewServer = false;
        } else {
          config.servers = config.servers.map(s =>
            s.id === editingServer!.id ? editingServer! : s
          );
        }

        // 이 서버로 전환 (SSH 재연결 + 서버 설정 로드)
        const newConfig: AppConfig = await invoke("switch_server", {
          servers: config.servers,
          serverId: editingServer.id,
        });
        config.active_server = editingServer.id;
        config.cms_config = newConfig.cms_config;
        config.shortcuts = newConfig.shortcuts;
        url.set(config.cms_config.hugo_config.url);
        contentPaths.set(config.cms_config.hugo_config.content_paths);
        hiddenPath.set(config.cms_config.hugo_config.hidden_path);
        activeServerName.set(editingServer.name || editingServer.ssh_config.host);
        refreshShortcutEntries();
        onServerSwitch();
      } catch (error) {
        console.error("Failed to connect with new SSH settings:", error);
        addToast("Failed to connect. Check SSH settings.");
        isSwitching = false;
        return; // 탭 전환 취소
      } finally {
        isSwitching = false;
      }
    }

    editTab = tab;
  }

  async function saveServer() {
    if (!editingServer) return;
    cancelRecording();

    if (!config.servers) config.servers = [];

    if (isNewServer) {
      config.servers = [...config.servers, editingServer];
      if (config.servers.length === 1) {
        config.active_server = editingServer.id;
      }
    } else {
      config.servers = config.servers.map(s =>
        s.id === editingServer!.id ? editingServer! : s
      );
    }

    // 저장
    try {
      await invoke("save_config", { config });
      await loadConfig();
      addToast("Settings saved.", "success");
      onServerSwitch();
    } catch (error) {
      console.error("Failed to save config:", error);
      addToast("Failed to save settings.");
    }

    view = "list";
    editingServer = null;
  }

  function confirmDeleteServer(id: string) {
    deletingServerId = id;
  }

  function proceedDeleteServer(confirmed: boolean) {
    if (confirmed && deletingServerId) {
      if (!config.servers) return;
      config.servers = config.servers.filter(s => s.id !== deletingServerId);
      if (config.active_server === deletingServerId) {
        config.active_server = config.servers.length > 0 ? config.servers[0].id : "";
      }
      invoke("save_config", { config }).catch(() => {});
    }
    deletingServerId = null;
  }

  // ── Shortcuts ──

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
              shortcut: trigger.content.shortcut,
              description: `${p.manifest.name} - ${trigger.content.label}`,
            });
          }
        }
      }
      pluginShortcutDefs.set(defs);
      buildShortcutMap();
      refreshShortcutEntries();
    } catch (_) {}
  }

  $: if (view === "edit" && editTab === "shortcuts") {
    loadPluginShortcuts();
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
    if (event.key === "Escape") { cancelRecording(); return; }
    const shortcutStr = eventToShortcutString(event);
    if (!shortcutStr) return;
    if (!config.shortcuts) config.shortcuts = {};
    const entry = shortcutEntries.find(e => e.id === recordingAction);
    if (!entry) return;
    let keys = [...entry.shortcuts];
    if (recordingIndex >= 0 && recordingIndex < keys.length) {
      keys[recordingIndex] = shortcutStr;
    } else {
      keys.push(shortcutStr);
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
    config.shortcuts[actionId] = keys.length === 0 ? [] : keys;
    refreshShortcutEntries();
  }

  function resetShortcut(actionId: string) {
    if (!config.shortcuts) return;
    delete config.shortcuts[actionId];
    config.shortcuts = { ...config.shortcuts };
    refreshShortcutEntries();
  }
</script>

<svelte:window on:keydown={handleShortcutRecord} />

<Popup {show} {isLoading} closePopup={() => { if (!isSetupRunning) closeSettings(); }}>
  {#if view === "list"}
    <!-- ═══ Server List View ═══ -->
    <h3 class="text-lg font-bold">Servers</h3>
    <div class="space-y-3 max-h-96 overflow-y-auto">
      {#each config.servers ?? [] as server (server.id)}
        <div class="server-card" class:server-active={server.id === config.active_server && isConnected} class:server-disconnected={server.id === config.active_server && !isConnected}>
          <div class="server-card-header">
            <button
              class="server-radio"
              class:selected={server.id === config.active_server && isConnected}
              class:disconnected={server.id === config.active_server && !isConnected}
              on:click={() => selectServer(server.id)}
              disabled={isSwitching}
              title="Set as active server"
            >
              {#if server.id === config.active_server}
                <div class="radio-dot" class:dot-disconnected={!isConnected}></div>
              {/if}
            </button>
            <div class="server-info">
              <span class="server-name">{server.name || server.ssh_config.host}</span>
              <span class="server-host">{server.ssh_config.host}:{server.ssh_config.port || "22"} · {server.ssh_config.username}</span>
            </div>
            <div class="server-actions">
              {#if server.id === config.active_server && !isConnected}
                <button class="server-action-btn reconnect" on:click={reconnectServer} disabled={isSwitching} title="Reconnect">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                  </svg>
                </button>
              {/if}
              <button class="server-action-btn" on:click={() => openEditServer(server)} title="Edit">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                  <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
                </svg>
              </button>
              <button class="server-action-btn delete" on:click={() => confirmDeleteServer(server.id)} title="Delete">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6"/>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                </svg>
              </button>
            </div>
          </div>
          {#if deletingServerId === server.id}
            <div class="confirm-box">
              <p class="text-sm">Delete this server?</p>
              <div class="flex justify-end gap-2 mt-2">
                <button class="px-3 py-1 rounded text-xs btn-danger" on:click={() => proceedDeleteServer(true)}>Delete</button>
                <button class="px-3 py-1 rounded text-xs btn-cancel" on:click={() => proceedDeleteServer(false)}>Cancel</button>
              </div>
            </div>
          {/if}
        </div>
      {/each}

      {#if !config.servers || config.servers.length === 0}
        <div class="empty-state">No servers configured.</div>
      {/if}

      <button class="add-server-btn" on:click={openAddServer}>
        + Add Server
      </button>
    </div>

  {:else if view === "edit" && editingServer}
    <!-- ═══ Server Edit View ═══ -->
    <div class="edit-header">
      <button class="back-btn" on:click={backToList} title="Back">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6"/>
        </svg>
      </button>
      <span class="edit-title">{isNewServer ? "Add Server" : editingServer.name || "Edit Server"}</span>
    </div>

    <!-- 탭 버튼 -->
    <div class="flex space-x-4">
      <button class="tab-button" class:active={editTab === "ssh"} on:click={() => switchEditTab("ssh")} disabled={isSwitching}>
        SSH
      </button>
      <button class="tab-button" class:active={editTab === "hugo"} on:click={() => switchEditTab("hugo")} disabled={isSwitching}>
        Hugo
      </button>
      <button class="tab-button" class:active={editTab === "shortcuts"} on:click={() => switchEditTab("shortcuts")} disabled={isSwitching}>
        Shortcuts
      </button>
    </div>

    <!-- 탭 콘텐츠 -->
    <div class="max-h-72 overflow-y-auto">
      {#if editTab === "ssh"}
        <div class="space-y-3">
          <DynamicField config={asFields(editingServer)} configKey="name" />
          {#each Object.keys(editingServer.ssh_config) as key}
            <DynamicField config={asFields(editingServer.ssh_config)} configKey={key} />
          {/each}
        </div>

      {:else if editTab === "hugo"}
        <div class="space-y-3">
          <HugoSetup bind:config bind:isSetupRunning />
          {#each Object.keys(config.cms_config.hugo_config).filter(k => k !== 'content_paths') as key}
            <DynamicField config={asFields(config.cms_config.hugo_config)} configKey={key} />
          {/each}
          <div class="flex items-center space-x-2">
            <label class="block min-w-[120px]" for="content-paths-input">content_paths</label>
            <input
              id="content-paths-input"
              class="flex-1 p-2 border rounded"
              value={config.cms_config.hugo_config.content_paths.join(', ')}
              on:change={(e) => {
                config.cms_config.hugo_config.content_paths = e.currentTarget.value
                  .split(',')
                  .map(s => s.trim())
                  .filter(s => s.length > 0);
                config = config;
              }}
              placeholder="posts, projects"
            />
          </div>
        </div>

      {:else if editTab === "shortcuts"}
        <div class="space-y-3">
          {#each shortcutEntries as entry}
            <div class="shortcut-row">
              <div class="shortcut-label" title="{entry.description} ({entry.id})">
                <span class="font-medium text-sm">{entry.description}</span>
              </div>
              <div class="shortcut-keys">
                {#each entry.shortcuts as key, i}
                  {#if recordingAction === entry.id && recordingIndex === i}
                    <span class="shortcut-badge recording">Press key...</span>
                  {:else}
                    <button class="shortcut-badge" on:click={() => startRecording(entry.id, i)} title="Click to change">{key}</button>
                    <button class="shortcut-remove" on:click={() => removeShortcut(entry.id, i)} title="Remove">&times;</button>
                  {/if}
                {/each}
                {#if recordingAction === entry.id && recordingIndex === -1}
                  <span class="shortcut-badge recording">Press key...</span>
                {:else}
                  <button class="shortcut-add" on:click={() => startRecording(entry.id, -1)} title="Add shortcut">+</button>
                {/if}
                {#if config.shortcuts && entry.id in config.shortcuts}
                  <button class="shortcut-reset" on:click={() => resetShortcut(entry.id)} title="Reset to default">Reset</button>
                {/if}
              </div>
            </div>
          {/each}
          {#if recordingAction}
            <p class="text-xs opacity-50">Press a key combination to assign, or Escape to cancel.</p>
          {/if}
        </div>
      {/if}
    </div>

    <button class="save-button" on:click={saveServer} disabled={isSetupRunning}>
      Save
    </button>
  {/if}
</Popup>

<style>
  /* ── Server cards ── */

  .server-card {
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    padding: 0.75rem;
    transition: border-color 0.15s;
  }

  .server-card.server-active {
    border-color: var(--status-connected-color);
  }

  .server-card.server-disconnected {
    border-color: var(--error-color);
  }

  .confirm-box {
    margin-top: 0.5rem;
    padding: 0.75rem;
    border-radius: 0.375rem;
    border: 1px solid var(--confirm-box-border);
    background-color: var(--confirm-box-bg);
    color: var(--confirm-box-text);
  }

  .btn-danger { background-color: var(--btn-danger-bg); color: var(--btn-danger-text); }
  .btn-danger:hover { background-color: var(--btn-danger-hover-bg); }
  .btn-cancel { background-color: var(--btn-cancel-bg); color: var(--btn-cancel-text); }
  .btn-cancel:hover { background-color: var(--btn-cancel-hover-bg); }

  .server-action-btn.reconnect {
    color: var(--error-color);
    opacity: 0.8;
  }

  .server-action-btn.reconnect:hover {
    opacity: 1;
    color: var(--error-color);
  }

  .server-card-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .server-radio {
    width: 16px;
    height: 16px;
    min-width: 16px;
    border-radius: 50%;
    border: 2px solid var(--border-color);
    background: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    box-shadow: none;
  }

  .server-radio.selected {
    border-color: var(--status-connected-color);
  }

  .server-radio.disconnected {
    border-color: var(--error-color);
  }

  .radio-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--status-connected-color);
  }

  .radio-dot.dot-disconnected {
    background-color: var(--error-color);
  }

  .server-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .server-name {
    font-weight: 500;
    font-size: 0.875rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .server-host {
    font-size: 0.75rem;
    opacity: 0.6;
  }

  .server-actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .server-action-btn {
    padding: 0.25rem;
    border: none;
    background: none;
    cursor: pointer;
    opacity: 0.4;
    border-radius: 0.25rem;
    box-shadow: none;
  }

  .server-action-btn:hover {
    opacity: 1;
    background-color: var(--button-hover-bg-color);
  }

  .server-action-btn.delete:hover {
    color: var(--error-color);
  }

  .empty-state {
    text-align: center;
    opacity: 0.4;
    font-size: 0.85rem;
    padding: 1rem;
  }

  .add-server-btn {
    width: 100%;
    padding: 0.5rem;
    border: 1px dashed var(--border-color);
    border-radius: 0.5rem;
    background: none;
    cursor: pointer;
    opacity: 0.6;
    font-size: 0.85rem;
    box-shadow: none;
  }

  .add-server-btn:hover {
    opacity: 1;
    background-color: var(--button-hover-bg-color);
  }

  /* ── Edit view ── */

  .edit-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .back-btn {
    padding: 0.25rem;
    border: none;
    background: none;
    cursor: pointer;
    opacity: 0.6;
    border-radius: 0.25rem;
    box-shadow: none;
    display: flex;
    align-items: center;
  }

  .back-btn:hover {
    opacity: 1;
    background-color: var(--button-hover-bg-color);
  }

  .edit-title {
    font-weight: 600;
    font-size: 0.9rem;
  }

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
    margin-top: 0.5rem;
  }

  /* ── Shortcuts ── */

  .shortcut-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    border-radius: 0.375rem;
    background-color: var(--sidebar-bg-color);
    gap: 0.5rem;
  }

  .shortcut-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .shortcut-keys {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
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
    box-shadow: none;
  }

  .shortcut-badge:hover { border-color: var(--border-color); }

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
    box-shadow: none;
  }

  .shortcut-remove:hover { opacity: 1; color: var(--shortcut-remove-hover); }

  .shortcut-add {
    font-size: 0.75rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    opacity: 0.4;
    cursor: pointer;
    border: 1px dashed var(--border-color);
    background: none;
    box-shadow: none;
  }

  .shortcut-add:hover { opacity: 1; }

  .shortcut-reset {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    opacity: 0.4;
    cursor: pointer;
    border: none;
    background: none;
    box-shadow: none;
    text-decoration: underline;
  }

  .shortcut-reset:hover { opacity: 1; }
</style>
