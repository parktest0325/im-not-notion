<script lang="ts">
  import { onMount, onDestroy, setContext } from "svelte";
  import { get } from "svelte/store";
  import { listen } from "@tauri-apps/api/event";
  import MainContent from "./content/MainContent.svelte";
  import { refreshList } from "./sidebar/FileControlSection.svelte";
  import Sidebar from "./sidebar/Sidebar.svelte";
  import TopBar from "./topbar/TopBar.svelte";
  import { GLOBAL_FUNCTIONS } from "./context";
  import Toast from "./component/Toast.svelte";
  import PluginResultPopup from "./sidebar/PluginResultPopup.svelte";
  import { handleShortcutEvent, buildShortcutMap, registerAction } from "./shortcut";
  import { addToast, selectedCursor, isEditingFileName, isEditingContent, renamingPath } from "./stores";
  import type { PluginAction } from "./types/setting";
  import "./theme"; // Initialize theme on app startup

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

  // Hook 결과의 PluginAction 수신
  let showHookResult = false;
  let hookResultTitle = "";
  let hookResultBody = "";
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    unlisten = await listen<PluginAction>("plugin-hook-action", (event) => {
      const action = event.payload;
      if (action.type === "toast" && action.content) {
        addToast(
          action.content.message,
          action.content.toast_type === "success" ? "success" : "error"
        );
      } else if (action.type === "refresh_tree") {
        refreshList();
      } else if (action.type === "show_result" && action.content) {
        hookResultTitle = action.content.title;
        hookResultBody = action.content.body;
        showHookResult = true;
      }
    });
  });

  onDestroy(() => {
    unlisten?.();
  });
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

<PluginResultPopup
  show={showHookResult}
  title={hookResultTitle}
  body={hookResultBody}
  onClose={() => { showHookResult = false; }}
/>