<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import MdArrowBack from "svelte-icons/md/MdArrowBack.svelte";
  import SettingsButton from "./SettingsButton.svelte";
</script>

<div
  class="sidebar"
  class:open={!isMenuOpen}
  style="background-color:var(--pastel-blue);"
>
  <div class="sidebar-section logo-section">
    {#if isMenuOpen}
      <img src="inn.svg" alt="IM NOT NOTION!" />
      <button on:click={toggleMenu} class="menu-button">
        <div class="icon">
          <MdArrowBack />
        </div>
      </button>
    {/if}
  </div>
  <div class="sidebar-section tools-section">
    <SettingsButton />
  </div>
  <div class="sidebar-section file-list-section">
    <!-- 파일 리스트 출력될 부분 -->
    <ul>
      <li>File 1</li>
      <li>File 2</li>
      <!-- 파일 목록 -->
    </ul>
  </div>
</div>

<style>
  :root {
    --pastel-blue: #a3cef1;
  }
  .sidebar {
    width: 250px;
    transition: width 0.3s;
    overflow: hidden;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .sidebar.open {
    width: 0px;
  }
  .sidebar-section {
    padding: 10px;
  }
  .logo-section {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .tools-section {
    display: flex;
    justify-content: space-around;
  }
  .tool-button {
    background: none;
    border: none;
    cursor: pointer;
  }
  .file-list-section {
    flex-grow: 1;
    overflow-y: auto;
  }
  .logo {
    max-width: 100%;
    height: auto;
  }

  .menu-button {
    border: none;
    background: none;
    cursor: pointer;
    padding: 0.5em;
    font-size: 1em;
  }
  .icon {
    width: 24px;
    height: 24px;
  }
</style>
