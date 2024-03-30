<script lang="ts">
  import { invoke } from "@tauri-apps/api";

  export let show: boolean;
  export let closeSettings: () => void;

  let config: AppConfig;
  let isLoading = true; // 로딩 상태 추가
  let activeTab = "ssh"; // 'ssh' 또는 'hugo'가 될 수 있음
  let hidePassword = true;

  $: if (show) {
    // show가 true일 때만 설정 로드
    loadConfig();
  }

  async function loadConfig() {
    isLoading = true; // 로딩 시작
    try {
      config = await invoke("load_config");
    } catch (error) {
      console.log("Failed to load config:", error);
      config = {
        ssh_config: {
          host: "",
          port: "",
          username: "",
          password: "",
          key_path: "",
        },
        hugo_config: {
          trashcan_path: "",
          hugo_cmd_path: "",
          base_path: "",
          content_path: "",
          image_path: "",
          config_path: "",
          layout_path: "",
        },
      };
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

{#if show}
  <div
    class="fixed inset-0 bg-black bg-opacity-50 flex justify-center items-center p-4"
  >
    {#if isLoading}
      <p>Loading...</p>
    {:else}
      <div class="bg-white rounded-lg shadow-xl p-6 w-full max-w-lg space-y-4">
        <!-- 탭 버튼 -->
        <div class="flex space-x-4">
          <button
            class="flex-1 py-2 px-4 rounded-lg focus:outline-none"
            class:bg-blue-500={activeTab === "ssh"}
            class:text-white={activeTab === "ssh"}
            on:click={() => (activeTab = "ssh")}
          >
            SSH Setting
          </button>
          <button
            class="flex-1 py-2 px-4 rounded-lg focus:outline-none"
            class:bg-blue-500={activeTab === "hugo"}
            class:text-white={activeTab === "hugo"}
            on:click={() => (activeTab = "hugo")}
          >
            Hugo Setting
          </button>
        </div>

        <!-- 설정 입력 필드 -->
        {#if activeTab === "ssh"}
          <div class="space-y-4">
            <!-- SSH 설정 필드 -->
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="ssh_client-host"
                >Host</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="ssh_client-host"
                bind:value={config.ssh_config.host}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="ssh_client-port"
                >Port</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="ssh_client-port"
                bind:value={config.ssh_config.port}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="ssh_client-username"
                >Username</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="ssh_client-username"
                bind:value={config.ssh_config.username}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="ssh_client-password"
                >Password</label
              >
              {#if hidePassword}
                <input
                  class="flex-1 p-2 border rounded"
                  id="ssh_client-password"
                  type="password"
                  bind:value={config.ssh_config.password}
                />
              {:else}
                <input
                  class="flex-1 p-2 border rounded"
                  id="ssh_client-password"
                  bind:value={config.ssh_config.password}
                />
              {/if}
              <button
                class="p-2 border rounded bg-blue-500 text-white"
                on:click={() => (hidePassword = !hidePassword)}
              >
                {hidePassword ? "Hide" : "Show"}
              </button>
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="ssh_client-key_path"
                >Key Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="ssh_client-key_path"
                bind:value={config.ssh_config.key_path}
              />
            </div>
          </div>
        {:else if activeTab === "hugo"}
          <div class="space-y-4">
            <!-- HUGO 설정 필드 -->
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="hugo_config-hugo_cmd_path"
                >Hugo Cmd Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="hugo_config-hugo_cmd_path"
                bind:value={config.hugo_config.hugo_cmd_path}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="hugo_config-base_path"
                >Base Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="hugo_config-base_path"
                bind:value={config.hugo_config.base_path}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="hugo_config-content_path"
                >Content Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="hugo_config-content_path"
                bind:value={config.hugo_config.content_path}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="hugo_config-image_path"
                >Image Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="hugo_config-image_path"
                bind:value={config.hugo_config.image_path}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="hugo_config-config_path"
                >Config Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="hugo_config-config_path"
                bind:value={config.hugo_config.config_path}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="hugo_config-layout_path"
                >Layout Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="hugo_config-layout_path"
                bind:value={config.hugo_config.layout_path}
              />
            </div>
            <div class="flex items-center space-x-2">
              <label class="block min-w-[120px]" for="hugo_config-trashcan_path"
                >Trashcan Path</label
              >
              <input
                class="flex-1 p-2 border rounded"
                id="hugo_config-trashcan_path"
                bind:value={config.hugo_config.trashcan_path}
              />
            </div>
          </div>
        {/if}

        <!-- 공용 저장 버튼 -->
        <button
          class="w-full py-2 px-4 bg-blue-500 text-white rounded-lg focus:outline-none"
          on:click={saveAndClose}
        >
          Save and Exit
        </button>
      </div>
    {/if}
  </div>
{/if}
