<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowBack from "svelte-icons/md/MdArrowBack.svelte";
  import Buttons from "./Buttons.svelte";
  import FileControlSection from "./FileControlSection.svelte";
  import LogoSVG from '../resource/LogoSVG.svelte';
  import { writable } from "svelte/store";

  let isConnected = writable(false);
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
  <div class="p-4 flex items-center">
    {#if $isConnected}
      <div class="flex items-center text-green-500">
        <div class="w-2 h-2 bg-green-500 rounded-full mr-2"></div>
        Connected
      </div>
    {:else}
      <div class="flex items-center text-red-500">
        <div class="w-2 h-2 bg-red-500 rounded-full mr-2"></div>
        Not Connected
      </div>
    {/if}
  </div>
</div>
