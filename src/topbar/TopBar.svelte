<script lang="ts">
    export let isMenuOpen: boolean;
    export let toggleMenu: () => void;
    import MdArrowForward from "svelte-icons/md/MdArrowForward.svelte";
    import { selectedFilePath } from "../stores";
  </script>
  
  <div class="top-bar p-4" style="background-color: var(--topbar-bg-color);">
    {#if !isMenuOpen}
      <button on:click={toggleMenu} class="w-6 h-6 mr-4">
        <MdArrowForward />
      </button>
    {/if}
    <span class="top-bar-title">{$selectedFilePath}</span>
  </div>
  