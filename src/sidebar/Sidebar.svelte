<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowBack from "svelte-icons/md/MdArrowBack.svelte";
  import Buttons from "./Buttons.svelte";
  import FileControlSection from "./FileControlSection.svelte";
  import LogoSVG from '../resource/LogoSVG.svelte';
  import { isConnected } from "../stores";
  import { currentTheme, cycleTheme } from "../theme";
</script>

<div
  class={`flex flex-col h-screen transition-all duration-300 overflow-hidden ${isMenuOpen ? "w-72" : "w-0"}`}
  style="flex-shrink: 0; background-color: var(--sidebar-bg-color);"
>
  <div class="flex items-center justify-between p-4" style="background-color: var(--sidebar-bg-color);">
    {#if isMenuOpen}
      <LogoSVG />
      <button on:click={toggleMenu} class="text-lg w-6 h-6">
        <MdArrowBack />
      </button>
    {/if}
  </div>
  <div class="p-4">
    <Buttons />
  </div>
  <div class="flex-grow overflow-y-auto p-4">
    <FileControlSection {isConnected} />
  </div>
  <div class="p-4 flex items-center justify-between">
    {#if $isConnected}
      <div class="flex items-center" style="color: var(--status-connected-color)">
        <div class="w-2 h-2 rounded-full mr-2" style="background-color: var(--status-connected-color)"></div>
        Connected
      </div>
    {:else}
      <div class="flex items-center" style="color: var(--status-disconnected-color)">
        <div class="w-2 h-2 rounded-full mr-2" style="background-color: var(--status-disconnected-color)"></div>
        Not Connected
      </div>
    {/if}
    <button
      on:click={cycleTheme}
      class="theme-btn"
      title={$currentTheme === "system" ? "Theme: System" : $currentTheme === "light" ? "Theme: Light" : "Theme: Dark"}
    >
      {#if $currentTheme === "system"}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>
        </svg>
      {:else if $currentTheme === "light"}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
        </svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
      {/if}
    </button>
  </div>
</div>

<style>
  .theme-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 0.25rem;
    border: 1px solid var(--border-color);
    background: none;
    cursor: pointer;
    padding: 0;
    box-shadow: none;
  }
  .theme-btn:hover {
    background-color: var(--button-hover-bg-color);
    border-color: var(--hover-border-color);
  }
</style>
