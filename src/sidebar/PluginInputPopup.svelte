<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../stores";
  import { dispatchPluginActions } from "../pluginActions";
  import type { PluginManifest, InputField, PluginResult } from "../types/setting";

  export let show: boolean;
  export let plugin: PluginManifest | null = null;
  export let inputFields: InputField[] = [];
  export let onClose: () => void;
  export let onShowResult: (title: string, body: string, pages?: any[]) => void = () => {};
  export let onDownloadFiles: (items: any[]) => void = () => {};
  // Called when run_plugin settles (success or error). PluginPanel uses it
  // to clear lingering progress/prompt state so the modal disappears.
  export let onComplete: () => void = () => {};

  let values: Record<string, string | boolean> = {};
  let isExecuting = false;

  // 기본값 초기화는 팝업이 "열릴 때 한 번"만. 조건 없이 반응형으로 돌리면
  // values 변경(=타이핑) 때마다 블록이 다시 돌아 입력값이 지워진다.
  let initKey = "";
  $: if (show) {
    const key = `${plugin?.name ?? ""}|${inputFields.map((f) => f.name).join(",")}`;
    if (key !== initKey) {
      initKey = key;
      const next: Record<string, string | boolean> = {};
      for (const field of inputFields) {
        next[field.name] =
          field.type === "boolean" ? field.default === "true" : (field.default ?? "");
      }
      values = next;
    }
  } else if (initKey) {
    initKey = ""; // 닫히면 다음에 열 때 다시 초기화
  }

  async function executePlugin() {
    if (!plugin) return;
    isExecuting = true;
    try {
      // values는 각 input의 on:input/on:change 핸들러가 최신으로 유지한다.
      // (getElementById로 DOM을 다시 읽으면 전역 id 충돌 시 엉뚱한 값을 읽는다)
      const formData: Record<string, string | boolean> = { trigger: "manual", ...values };
      const inputJson = JSON.stringify(formData);
      const result: PluginResult = await invoke("run_plugin", {
        name: plugin.name,
        input: inputJson,
      });

      if (result.success) {
        addToast(result.message ?? "Plugin executed.", "success");
      } else {
        addToast(result.error ?? "Plugin failed.");
      }

      dispatchPluginActions(result.actions, {
        onShowResult,
        onDownloadFiles,
      });

      onClose();
    } catch (error) {
      console.error("Plugin execution failed:", error);
      addToast("Plugin execution failed.");
    } finally {
      isExecuting = false;
      onComplete();
    }
  }
</script>

{#if show}
  <div class="fixed inset-0 flex justify-center items-center p-4 input-overlay">
    <div class="input-popup-content">
      {#if plugin}
        <div class="input-popup-header">
          <h3 class="text-lg font-bold">{plugin.name}</h3>
          <button class="input-popup-close" on:click={onClose} title="Close" aria-label="Close">✕</button>
        </div>
        <p class="text-sm opacity-70">{plugin.description}</p>

        <div class="space-y-3">
          {#each inputFields as field}
            <div>
              {#if field.type === "boolean"}
                <label class="flex items-center gap-2 text-sm cursor-pointer" for={field.name}>
                  <input
                    id={field.name}
                    type="checkbox"
                    bind:checked={values[field.name]}
                  />
                  {field.label}
                </label>
              {:else}
                <label class="block text-sm mb-1" for={field.name}>{field.label}</label>
                <input
                  id={field.name}
                  type={field.type === "password" ? "password" : "text"}
                  class="w-full p-2 rounded border"
                  style="background-color: var(--input-bg-color); border-color: var(--border-color);"
                  value={values[field.name]}
                  on:input={(e) => { values[field.name] = e.currentTarget.value; }}
                  placeholder={field.default ?? ""}
                />
              {/if}
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
    </div>
  </div>
{/if}

<style>
  .input-overlay {
    background-color: var(--overlay-bg-color);
    z-index: 1100;
  }
  .input-popup-content {
    background-color: var(--popup-bg-color);
    color: var(--popup-text-color);
    padding: 1.5rem;
    border-radius: 0.5rem;
    box-shadow: var(--shadow-popup);
    width: 100%;
    max-width: 32rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .input-popup-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .input-popup-close {
    background: none;
    border: none;
    color: inherit;
    opacity: 0.5;
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    font-size: 1rem;
    line-height: 1;
    border-radius: 0.25rem;
    transition: opacity 0.15s ease, background-color 0.15s ease;
  }
  .input-popup-close:hover {
    opacity: 1;
    background-color: var(--button-active-bg-color);
  }
</style>
