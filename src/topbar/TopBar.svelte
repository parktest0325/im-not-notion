<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowForward from "svelte-icons/md/MdArrowForward.svelte";
  import DiIe from 'svelte-icons/di/DiIe.svelte'
  import { relativeFilePath, url, contentPath, hiddenPath, fullFilePath, type GlobalFunctions, GLOBAL_FUNCTIONS } from "../stores";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-shell";
  import { getContext, onMount } from "svelte";
  import type { AppConfig } from "../types/setting";

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
    // if (!$relativeFilePath || $relativeFilePath.endsWith('_index.md')) return;
    try {
      isHidden = await invoke("check_file_hidden", { path: $relativeFilePath });
      console.log(isHidden);
    } catch (error) {
      console.error("Failed to check hidden status:", error);
      isHidden = false; // 오류 시 기본값 설정
    }
  }

  let config: AppConfig;

  async function toggleHidden() {
    if (!$relativeFilePath || isLoading) return;
    
    isLoading = true;
    try {
      await invoke("toggle_hidden_file", { path: $relativeFilePath, state: isHidden });
      isHidden = !isHidden;
      // // 토글 후 전체 파일 경로 갱신 없어도 잘만됨
      // const newPath = (isHidden ? `/${$hiddenPath}` : '') + `/${$contentPath}${$relativeFilePath}`;
      // fullFilePath.set(newPath);
      await refreshList();
    } catch (error) {
      console.error("Failed to toggle hidden status:", error);
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

  onMount(async () => {
    try {
      config = await invoke("get_config");
      url.set(config.cms_config.hugo_config.url);
      contentPath.set(config.cms_config.hugo_config.content_path);
      hiddenPath.set(config.cms_config.hugo_config.hidden_path);
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
    <span>{$fullFilePath}</span>
  </div>
  <div class="flex items-center gap-2">
    {#if $relativeFilePath && !$relativeFilePath.endsWith('_index.md')}
      <button 
        on:click={toggleHidden}
        class="px-3 py-1 text-sm rounded border transition-colors duration-200"
        class:bg-blue-100={!isHidden}
        class:text-blue-800={!isHidden}
        class:border-blue-300={!isHidden}
        class:hover:bg-blue-200={!isHidden}
        class:bg-orange-100={isHidden}
        class:text-orange-800={isHidden}
        class:border-orange-300={isHidden}
        class:hover:bg-orange-200={isHidden}
        class:dark:bg-blue-800={!isHidden}
        class:dark:text-blue-100={!isHidden}
        class:dark:border-blue-600={!isHidden}
        class:dark:hover:bg-blue-700={!isHidden}
        class:dark:bg-orange-800={isHidden}
        class:dark:text-orange-100={isHidden}
        class:dark:border-orange-600={isHidden}
        class:dark:hover:bg-orange-700={isHidden}
      >
        {isHidden ? "숨겨짐" : "표시중"}
      </button>
    {/if}
    <button on:click={handleOpenPage} class="w-6 h-6 border flex items-center justify-center">
      <DiIe />
    </button>
  </div>
</div>
