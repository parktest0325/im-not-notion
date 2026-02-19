<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import DynamicField from "../component/DynamicField.svelte";
  import HugoSetup from "./HugoSetup.svelte";
  import { createDefaultAppConfig, type AppConfig } from "../types/setting";
  import Popup from "../component/Popup.svelte";
  import { url, contentPath, hiddenPath } from "../stores";
  import { onMount } from "svelte";

  export let show: boolean;
  export let closeSettings: () => void;

  let config: AppConfig;
  let isLoading = true;
  let activeTab = "ssh";
  let isSetupRunning = false;

  onMount(loadConfig);

  $: if (show) {
    // show가 true일 때만 설정 로드
    loadConfig();
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
    } catch (error) {
      console.log("Failed to load config:", error);
      config = createDefaultAppConfig();
    } finally {
      isLoading = false; // 로딩 완료
    }
  }

  async function saveAndClose() {
    try {
      await invoke("save_config", { config });
      await loadConfig(); // 저장 후 최신 상태 로드
    } catch (error) {
      console.error(error);
    } finally {
      closeSettings();
    }
  }
</script>

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
  </div>

  <!-- 설정 입력 필드 -->
  {#if activeTab === "ssh"}
    <div class="space-y-4">
      {#each Object.keys(config.ssh_config) as key}
        <DynamicField config={config.ssh_config} configKey={key} />
      {/each}
    </div>
  {:else if activeTab === "hugo"}
    <div class="space-y-4">
      <HugoSetup bind:config bind:isSetupRunning />
      {#each Object.keys(config.cms_config.hugo_config) as key}
        <DynamicField config={config.cms_config.hugo_config} configKey={key} />
      {/each}
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
</style>
