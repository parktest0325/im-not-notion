<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowForward from "svelte-icons/md/MdArrowForward.svelte";
  import DiIe from 'svelte-icons/di/DiIe.svelte'
  import { relativeFilePath, url, contentPath, hiddenPath, fullFilePath, addToast } from "../stores";
  import { type GlobalFunctions, GLOBAL_FUNCTIONS } from "../context";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-shell";
  import { getContext } from "svelte";

  let isHidden = false;
  let isLoading = false;

  const { refreshList } = getContext<GlobalFunctions>(GLOBAL_FUNCTIONS);

  function handleOpenPage() {
    let cleanedPath = $fullFilePath
      .replace(/\.md$/, "")
      .replace(/\/_index$/, "")
      .toLowerCase();
    
    const fullUrl = new URL(cleanedPath, $url);
    open(fullUrl.toString().toLowerCase());
  }

  async function checkHidden() {
    try {
      isHidden = await invoke("check_file_hidden", { path: $relativeFilePath });
    } catch (error) {
      console.error("Failed to check hidden status:", error);
      isHidden = false;
      addToast("Failed to check hidden status.");
    }
  }

  async function toggleHidden() {
    if (!$relativeFilePath || isLoading) return;
    
    isLoading = true;
    try {
      await invoke("toggle_hidden_file", { path: $relativeFilePath, state: isHidden });
      isHidden = !isHidden;
      await refreshList();
      addToast(isHidden ? "File hidden." : "File visible.", "success");
    } catch (error) {
      console.error("Failed to toggle hidden status:", error);
      addToast("Failed to toggle hidden status.");
    } finally {
      isLoading = false;
    }
  }
  
  $: if ($relativeFilePath) {
    // 파일 선택 시 숨김 상태를 확인하고 전체 경로를 설정
    checkHidden();
    
    // relativeFilePath가 갱신되면 전체 파일 경로 갱신
    const newPath = (isHidden ? `/${$hiddenPath}` : '') + `/${$contentPath}${$relativeFilePath}`;
    fullFilePath.set(newPath);
  }


</script>

<div class="p-4 flex justify-between items-center" style="background-color: var(--topbar-bg-color);">
  <div class="flex items-center">
    {#if !isMenuOpen}
      <button on:click={toggleMenu} class="w-6 h-6 mr-4">
        <MdArrowForward />
      </button>
    {/if}
    <span>{$fullFilePath}</span>
  </div>
  <div class="flex items-center gap-2">
    {#if $relativeFilePath && !$relativeFilePath.endsWith('_index.md')}
      <button
        on:click={toggleHidden}
        class="px-3 py-1 text-sm rounded border transition-colors duration-200"
        class:btn-visible={!isHidden}
        class:btn-hidden={isHidden}
      >
        {isHidden ? "Show" : "Hide"}
      </button>
    {/if}
    <button on:click={handleOpenPage} class="w-6 h-6 border flex items-center justify-center">
      <DiIe />
    </button>
  </div>
</div>

<style>
  .btn-visible {
    background-color: var(--btn-visible-bg);
    color: var(--btn-visible-text);
    border-color: var(--btn-visible-border);
  }
  .btn-visible:hover {
    background-color: var(--btn-visible-hover-bg);
  }
  .btn-hidden {
    background-color: var(--btn-hidden-bg);
    color: var(--btn-hidden-text);
    border-color: var(--btn-hidden-border);
  }
  .btn-hidden:hover {
    background-color: var(--btn-hidden-hover-bg);
  }
</style>
