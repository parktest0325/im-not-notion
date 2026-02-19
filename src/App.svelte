<script lang="ts">
  import { setContext } from "svelte";
  import { get } from "svelte/store";
  import MainContent from "./content/MainContent.svelte";
  import { refreshList } from "./sidebar/FileControlSection.svelte";
  import Sidebar from "./sidebar/Sidebar.svelte";
  import TopBar from "./topbar/TopBar.svelte";
  import { GLOBAL_FUNCTIONS } from "./context";
  import Toast from "./component/Toast.svelte";
  import { handleShortcutEvent, buildShortcutMap, registerAction } from "./shortcut";
  import { selectedCursor, isEditingFileName, isEditingContent, renamingPath } from "./stores";

  let isMenuOpen: boolean = true;

  function toggleMenu(): void {
    isMenuOpen = !isMenuOpen;
  }

  // Build shortcut map with defaults (no client overrides or plugins yet — applied on config load)
  buildShortcutMap({}, []);

  // Register global rename action
  registerAction("rename", () => {
    const cursor = get(selectedCursor);
    if (cursor && !get(isEditingFileName) && !get(isEditingContent)) {
      renamingPath.set(cursor);
    }
  });

  setContext(GLOBAL_FUNCTIONS, { refreshList });
</script>

<svelte:window on:keydown={handleShortcutEvent} />

<div class="flex h-screen">
  <Sidebar {isMenuOpen} {toggleMenu} />
  <div class="flex-grow flex flex-col bg-maincontent">
    <TopBar {isMenuOpen} {toggleMenu} />
    <MainContent />
  </div>
</div>

<Toast />