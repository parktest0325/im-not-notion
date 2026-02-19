<script lang="ts">
  import { setContext } from "svelte";
  import MainContent from "./content/MainContent.svelte";
  import { refreshList } from "./sidebar/FileControlSection.svelte";
  import Sidebar from "./sidebar/Sidebar.svelte";
  import TopBar from "./topbar/TopBar.svelte";
  import { GLOBAL_FUNCTIONS } from "./stores";
  import Toast from "./component/Toast.svelte";

  let isMenuOpen: boolean = true;

  function toggleMenu(): void {
    isMenuOpen = !isMenuOpen;
  }
  
  setContext(GLOBAL_FUNCTIONS, { refreshList });
</script>

<div class="flex h-screen">
  <Sidebar {isMenuOpen} {toggleMenu} />
  <div class="flex-grow flex flex-col bg-maincontent">
    <TopBar {isMenuOpen} {toggleMenu} />
    <MainContent />
  </div>
</div>

<Toast />