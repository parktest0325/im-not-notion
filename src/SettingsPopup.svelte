<script lang="ts">
  import { invoke } from "@tauri-apps/api";
  export let show: boolean;
  export let closeSettings: () => void;

  let config: AppConfig;
  let isLoading = true; // 로딩 상태 추가

  $: if (show) {
    // show가 true일 때만 설정 로드
    loadConfig();
  }

  async function loadConfig() {
    isLoading = true; // 로딩 시작
    try {
      config = await invoke("load_config");
    } catch (error) {
      console.error("Failed to load config:", error);
    } finally {
      isLoading = false; // 로딩 완료
    }
  }

  async function saveAndClose() {
    try {
      await invoke("save_config", { config });
    } catch (error) {
      console.error("Failed to load config:", error);
    } finally {
      closeSettings();
    }
  }
</script>

{#if show}
  {#if isLoading}
    <p>Loading...</p>
    <!-- 로딩 인디케이터 표시 -->
  {:else}
    <div class="popup">
      <div class="popup-content">
        <div class="setting-item">
          <label for="ssh_client-host">Host</label>
          <input id="ssh_client-host" bind:value={config.ssh_client.host} />
        </div>
        <div class="setting-item">
          <label for="ssh_client-port">Port</label>
          <input id="ssh_client-port" bind:value={config.ssh_client.port} />
        </div>
        <div class="setting-item">
          <label for="ssh_client-username">Username</label>
          <input
            id="ssh_client-username"
            bind:value={config.ssh_client.username}
          />
        </div>
        <div class="setting-item">
          <label for="ssh_client-password">Password</label>
          <input
            id="ssh_client-password"
            type="password"
            bind:value={config.ssh_client.password}
          />
        </div>
        <div class="setting-item">
          <label for="ssh_client-key_path">Key Path</label>
          <input
            id="ssh_client-key_path"
            bind:value={config.ssh_client.key_path}
          />
        </div>
        <button on:click={saveAndClose}>저장하고 나가기</button>
      </div>
    </div>
  {/if}
{/if}

<style>
  .popup {
    /* 팝업 백그라운드 스타일 */
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .popup-content {
    /* 팝업 내용 스타일 */
    background: white;
    padding: 20px;
    border-radius: 5px;
    display: flex;
    flex-direction: column; /* 세로로 나열 */
  }
  .setting-item {
    margin-bottom: 10px; /* 각 설정 사이의 간격 */
  }
  .setting-item input {
    margin-right: 10px; /* 입력 필드와 버튼 사이의 간격 */
  }
</style>
