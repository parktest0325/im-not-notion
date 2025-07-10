<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import DynamicField from "../component/DynamicField.svelte";
  import { createDefaultAppConfig, type AppConfig } from "../types/setting";
  import Popup from "../component/Popup.svelte";

  export let show: boolean;
  export let closeSettings: () => void;

  let config: AppConfig;
  let isLoading = true; // 로딩 상태 추가
  let activeTab = "ssh"; // 'ssh' 또는 'hugo'가 될 수 있음

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
      await invoke("update_and_connect", { config });
    } catch (error) {
      console.error(error);
    } finally {
      closeSettings();
    }
  }
</script>

<Popup {show} {isLoading} closePopup={closeSettings}>
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
      {#each Object.keys(config.cms_config.hugo_config) as key}
        <DynamicField config={config.cms_config.hugo_config} configKey={key} />
      {/each}
    </div>
  {/if}

  <!-- 공용 저장 버튼 -->
  <button class="save-button" on:click={saveAndClose}>
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
