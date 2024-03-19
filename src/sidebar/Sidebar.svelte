<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowBack from "svelte-icons/md/MdArrowBack.svelte";
  import Buttons from "./Buttons.svelte";
  import FileControlSection from "./FileControlSection.svelte";
</script>

<div
  class={`flex flex-col h-screen bg-pastel-blue transition-all duration-300 overflow-hidden ${isMenuOpen ? "w-72" : "w-0"}`}
  style="flex-shrink: 0; background-color:var(--pastel-blue);"
>
  <div class="flex items-center justify-between p-4 bg-pastel-blue">
    {#if isMenuOpen}
      <img src="inn.svg" alt="IM NOT NOTION!" class="max-w-full h-auto" />
      <button on:click={toggleMenu} class="text-lg w-6 h-6">
        <MdArrowBack />
      </button>
    {/if}
  </div>
  <div class="p-4">
    <Buttons />
  </div>
  <div class="flex-grow overflow-y-auto p-4">
    <FileControlSection />
  </div>
</div>

<style>
  :root {
    --pastel-blue: #a3cef1;
  }
</style>
