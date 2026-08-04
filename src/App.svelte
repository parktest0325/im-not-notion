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
  import StatusBar from "./component/StatusBar.svelte";
  import PluginResultPopup from "./sidebar/PluginResultPopup.svelte";
  import PluginDownloadPopup from "./sidebar/PluginDownloadPopup.svelte";
  import { handleShortcutEvent, buildShortcutMap, registerAction } from "./shortcut";
  import { selectedCursor, isEditingFileName, isEditingContent, renamingPath } from "./stores";
  import { dispatchPluginActions } from "./pluginActions";
  import type { PluginAction, DownloadItem } from "./types/setting";
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
  let hookResultPages: any[] = [];
  let showHookDownload = false;
  let hookDownloadItems: DownloadItem[] = [];
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    unlisten = await listen<PluginAction>("plugin-hook-action", (event) => {
      dispatchPluginActions([event.payload], {
        onShowResult: (title, body, pages) => {
          hookResultTitle = title;
          hookResultBody = body;
          hookResultPages = pages ?? [];
          showHookResult = true;
        },
        onDownloadFiles: (items) => {
          hookDownloadItems = items;
          showHookDownload = true;
        },
      });
    });
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

<svelte:window on:keydown={handleShortcutEvent} />

<div class="flex flex-col h-screen">
  <div class="flex flex-1 min-h-0">
    <Sidebar {isMenuOpen} {toggleMenu} />
    <div class="flex-grow flex flex-col bg-maincontent min-w-0">
      <TopBar {isMenuOpen} {toggleMenu} />
      <MainContent />
    </div>
  </div>
  <StatusBar />
</div>

<Toast />

<PluginResultPopup
  show={showHookResult}
  title={hookResultTitle}
  body={hookResultBody}
  pages={hookResultPages}
  onClose={() => { showHookResult = false; }}
/>

<PluginDownloadPopup
  show={showHookDownload}
  items={hookDownloadItems}
  onClose={() => { showHookDownload = false; }}
/>