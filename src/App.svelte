<script lang="ts">
  import MainContent from "./MainContent.svelte";
  import Sidebar from "./Sidebar.svelte";
  import TopBar from "./TopBar.svelte";

  let isMenuOpen: boolean = true;

  function toggleMenu(): void {
    isMenuOpen = !isMenuOpen;
  }
</script>

<div class="app-container">
  <Sidebar {isMenuOpen} {toggleMenu} />
  <div class="main-content" style="background-color:var(--pastel-green);">
    <TopBar {isMenuOpen} {toggleMenu} />
    <MainContent />
  </div>
</div>

<style>
  :root {
    --pastel-green: #a4d4ae;
  }
  .app-container {
    display: flex;
    height: 100vh; /* 전체 높이 */
  }
  .main-content {
    flex-grow: 1;
    display: flex;
    flex-direction: column; /* 세로 방향 정렬 */
  }
</style>
