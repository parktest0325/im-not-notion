<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { addToast } from "../stores";
  import type { DownloadItem } from "../types/setting";

  export let show: boolean;
  export let items: DownloadItem[] = [];
  export let onClose: () => void;

  let checkedArr: boolean[] = [];
  let isDownloading = false;
  let progress = { current: 0, total: 0 };

  $: if (show && items.length > 0) {
    checkedArr = items.map((_, i) => i === 0);
    isDownloading = false;
    progress = { current: 0, total: 0 };
  }

  $: selectedCount = checkedArr.filter(Boolean).length;

  function toggle(index: number) {
    checkedArr[index] = !checkedArr[index];
    checkedArr = [...checkedArr];
  }

  function toggleAll() {
    const allChecked = selectedCount === items.length;
    checkedArr = items.map(() => !allChecked);
  }

  async function downloadSelected() {
    const selected = items.filter((_, i) => checkedArr[i]);
    if (selected.length === 0) {
      addToast("No files selected.");
      return;
    }

    const dir = await open({ directory: true, title: "Select download folder" });
    if (!dir) return;

    isDownloading = true;
    progress = { current: 0, total: selected.length };

    try {
      const batch: [string, string][] = selected.map(item => [
        item.path,
        `${dir}/${item.filename}`,
      ]);
      const results: ({ Ok: null } | { Err: string })[] = await invoke("download_remote_files", { items: batch });

      let succeeded = 0;
      for (let i = 0; i < results.length; i++) {
        if ("Err" in results[i]) {
          console.error(`Download failed: ${selected[i].filename}`, results[i]);
          addToast(`Failed: ${selected[i].filename}`);
        } else {
          succeeded++;
        }
        progress.current = i + 1;
        progress = progress;
      }

      if (succeeded > 0) {
        addToast(`Downloaded ${succeeded} file(s).`, "success");
      }
    } catch (error) {
      console.error("Batch download failed:", error);
      addToast("Download failed.");
    }

    isDownloading = false;
    onClose();
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="fixed inset-0 flex justify-center items-center p-4 dl-overlay" on:click|self={onClose}>
    <div class="dl-popup">
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-bold">Download Files</h3>
        <button class="text-xs opacity-50" on:click={toggleAll}>
          {selectedCount === items.length ? "Deselect all" : "Select all"}
        </button>
      </div>

      <div class="dl-list">
        {#each items as item, i}
          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div class="dl-item" on:click={() => toggle(i)}>
            <input
              type="checkbox"
              checked={checkedArr[i]}
              on:click|stopPropagation={() => toggle(i)}
            />
            <svg class="dl-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 8v13H3V8"/><path d="M1 3h22v5H1z"/><path d="M10 12h4"/>
            </svg>
            <span class="dl-name">{item.filename}</span>
            <span class="dl-size">{item.size}</span>
          </div>
        {/each}
      </div>

      {#if isDownloading}
        <div class="dl-progress">
          <div class="dl-progress-bar" style="width: {(progress.current / progress.total) * 100}%"></div>
        </div>
        <p class="text-xs text-center opacity-60">{progress.current} / {progress.total}</p>
      {/if}

      <div class="flex gap-2">
        <button
          class="flex-1 p-2 rounded"
          style="background-color: var(--button-active-bg-color);"
          on:click={downloadSelected}
          disabled={isDownloading || selectedCount === 0}
        >
          {isDownloading ? "Downloading..." : `Download (${selectedCount})`}
        </button>
        <button
          class="p-2 rounded opacity-60"
          on:click={onClose}
          disabled={isDownloading}
        >Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dl-overlay {
    background-color: var(--overlay-bg-color);
    z-index: 1100;
  }
  .dl-popup {
    background-color: var(--popup-bg-color);
    color: var(--popup-text-color);
    padding: 1.5rem;
    border-radius: 0.5rem;
    box-shadow: var(--shadow-popup);
    width: 100%;
    max-width: 28rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .dl-list {
    max-height: 50vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .dl-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.85rem;
    border: 1px solid var(--border-color);
    background-color: var(--sidebar-bg-color);
  }
  .dl-item:hover {
    background-color: var(--button-active-bg-color);
  }
  .dl-icon {
    flex-shrink: 0;
    opacity: 0.6;
  }
  .dl-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dl-size {
    font-size: 0.75rem;
    opacity: 0.5;
    flex-shrink: 0;
  }
  .dl-progress {
    height: 4px;
    border-radius: 2px;
    background-color: var(--border-color);
    overflow: hidden;
  }
  .dl-progress-bar {
    height: 100%;
    background-color: var(--button-active-bg-color);
    transition: width 0.3s;
  }
</style>
