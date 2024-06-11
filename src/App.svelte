<script lang="ts">
  import MainContent from "./content/MainContent.svelte";
  import Sidebar from "./sidebar/Sidebar.svelte";
  import TopBar from "./topbar/TopBar.svelte";

  let isMenuOpen: boolean = true;

  function toggleMenu(): void {
    isMenuOpen = !isMenuOpen;
  }
</script>

<div class="flex h-screen">
  <Sidebar {isMenuOpen} {toggleMenu} />
  <div class="flex-grow flex flex-col bg-maincontent">
    <TopBar {isMenuOpen} {toggleMenu} />
    <MainContent />
  </div>
</div>