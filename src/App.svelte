<script lang="ts">
  import MainContent from "./MainContent.svelte";
  import Sidebar from "./Sidebar.svelte";
  import TopBar from "./TopBar.svelte";

  let isMenuOpen: boolean = true;

  function toggleMenu(): void {
    isMenuOpen = !isMenuOpen;
  }
</script>

<div class="flex h-screen">
  <Sidebar {isMenuOpen} {toggleMenu} />
  <div class="flex-grow flex flex-col bg-pastel-green">
    <TopBar {isMenuOpen} {toggleMenu} />
    <MainContent />
  </div>
</div>

<style>
  /* Tailwind CSS에서 커스텀 색상을 정의하는 방법 */
  .bg-pastel-green {
    --tw-bg-opacity: 1;
    background-color: rgba(164, 212, 174, var(--tw-bg-opacity));
  }
</style>
