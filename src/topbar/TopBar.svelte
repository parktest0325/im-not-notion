<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowForward from "svelte-icons/md/MdArrowForward.svelte";
  import DiIe from 'svelte-icons/di/DiIe.svelte'
  import { selectedFilePath, url, contentPath } from "../stores";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-shell";
  import { onMount } from "svelte";
  import type { AppConfig } from "../types/setting";
  
  let isHidden = false;
  let isLoading = false;

  function handleOpenPage() {
    let cleanedPath = $selectedFilePath
      .replace(/\.md$/, "")
      .replace(/\/_index$/, "")
      .toLowerCase();

    // Hidden 파일이면 /Hidden/ 붙이고, 일반 파일이면 contentPath 붙이기
    const urlPath = isHidden ? `/Hidden${cleanedPath}` : `/${$contentPath}${cleanedPath}`;
    
    const fullUrl = new URL(urlPath, $url);
    open(fullUrl.toString().toLowerCase());
  }

  let config: AppConfig;

  async function checkHidden() {
    if (!$selectedFilePath || $selectedFilePath.endsWith('_index.md')) return;
    
    try {
      isHidden = await invoke("check_file_hidden", {
        path: $selectedFilePath,
      });
    } catch (error) {
      console.error("Failed to check hidden status:", error);
      isHidden = false; // 오류 시 기본값으로 설정
    }
  }
  
  async function toggleHidden() {
    if (!$selectedFilePath || isLoading) return;
    
    isLoading = true;
    try {
      if (isHidden) {
        await invoke("show_file", { path: $selectedFilePath });
        isHidden = false;
      } else {
        await invoke("hide_file", { path: $selectedFilePath });
        isHidden = true;
      }
    } catch (error) {
      console.error("Failed to toggle hidden status:", error);
    } finally {
      isLoading = false;
    }
  }
  
  $: if ($selectedFilePath) {
    isHidden = false; // 파일 변경 시 상태 초기화
    checkHidden();
  }

  onMount(async () => {
    try {
      config = await invoke("get_config");
      url.set(config.cms_config.hugo_config.url);
      contentPath.set(config.cms_config.hugo_config.content_path);
    } catch (error) {
      console.error("Failed to get config:", error);
    }
  });
</script>

<div class="p-4 flex justify-between items-center" style="background-color: var(--topbar-bg-color);">
  <div class="flex items-center">
    {#if !isMenuOpen}
      <button on:click={toggleMenu} class="w-6 h-6 mr-4">
        <MdArrowForward />
      </button>
    {/if}
    <span>{isHidden ? `/Hidden${$selectedFilePath}` : `/${$contentPath}${$selectedFilePath}`}</span>
  </div>
  <div class="flex items-center gap-2">
    {#if $selectedFilePath && !$selectedFilePath.endsWith('_index.md')}
      <button 
        on:click={toggleHidden}
        disabled={isLoading}
        class="px-3 py-1 text-sm rounded border transition-colors duration-200"
        class:bg-blue-100={!isHidden}
        class:text-blue-800={!isHidden}
        class:border-blue-300={!isHidden}
        class:hover:bg-blue-200={!isHidden && !isLoading}
        class:bg-orange-100={isHidden}
        class:text-orange-800={isHidden}
        class:border-orange-300={isHidden}
        class:hover:bg-orange-200={isHidden && !isLoading}
        class:dark:bg-blue-800={!isHidden}
        class:dark:text-blue-100={!isHidden}
        class:dark:border-blue-600={!isHidden}
        class:dark:hover:bg-blue-700={!isHidden && !isLoading}
        class:dark:bg-orange-800={isHidden}
        class:dark:text-orange-100={isHidden}
        class:dark:border-orange-600={isHidden}
        class:dark:hover:bg-orange-700={isHidden && !isLoading}
        class:opacity-50={isLoading}
        class:cursor-not-allowed={isLoading}
      >
        {isHidden ? "숨겨짐" : "표시중"}
      </button>
    {/if}
    <button on:click={handleOpenPage} class="w-6 h-6 border flex items-center justify-center">
      <DiIe />
    </button>
  </div>
</div>
