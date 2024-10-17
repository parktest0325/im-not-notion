<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowForward from "svelte-icons/md/MdArrowForward.svelte";
  import DiIe from 'svelte-icons/di/DiIe.svelte'
  import { selectedFilePath } from "../stores";
  import { invoke } from "@tauri-apps/api";
  import { onMount } from "svelte";
  import { type AppConfig } from "../types/setting";

  function handleOpenPage() {
    let cleanedPath = $selectedFilePath.replace(/\.md$/, "").replace(/\/_index$/, "");
    const fullUrl = new URL(`${contentPath}${cleanedPath}`, url);
    window.open(fullUrl.toString(), "_blank");
  }

  let config: AppConfig;
  let url: string = "https://www.naver.com";
  let contentPath: string = "/content/";
  onMount(async () => {
    try {
      config = await invoke("get_config");
      url = config.cms_config.hugo_config.url;
      contentPath = config.cms_config.hugo_config.content_path;
    } catch (error) {
      console.error("Failed to get config:", error);
    }
  });
</script>

<div class="top-bar p-4 flex justify-between items-center" style="background-color: var(--topbar-bg-color);">
  <div class="flex items-center">
    {#if !isMenuOpen}
      <button on:click={toggleMenu} class="w-6 h-6 mr-4">
        <MdArrowForward />
      </button>
    {/if}
    <span class="top-bar-title pr-4">{$selectedFilePath}</span>
  </div>
  <button on:click={handleOpenPage} class="w-6 h-6 border flex items-center justify-center" style="border-radius: 0;">
    <DiIe />
  </button>
</div>
