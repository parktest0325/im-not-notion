<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import type {
    ConflictItem, ConflictPolicy, FsEntry, TransferProgress,
  } from "../types/explorer";
  import { addToast } from "../stores";
  import ConflictModal from "./ConflictModal.svelte";
  import ProgressModal from "./ProgressModal.svelte";
  import ConfirmModal from "./ConfirmModal.svelte";

  type Item = { path: string; name: string; is_dir: boolean };

  export let show: boolean;
  export let closeExplorer: () => void;

  // ── State ──
  let cwd = "";
  let entries: FsEntry[] = [];
  let loading = false;
  let selected = new Set<string>();
  let lastClickedName: string | null = null;
  let contextMenu: { x: number; y: number; name: string } | null = null;
  let renamingName: string | null = null;
  let renameValue = "";
  let isDragOver = false;

  let downloadPath = "";
  let showSettings = false;
  let editingDownloadPath = "";

  // Delete confirmation
  let pendingDelete: Item[] = [];

  // Transfer
  let pendingItems: Item[] = [];
  let pendingTargetDir = "";
  let pendingDirection: "upload" | "download" | null = null;
  let conflicts: ConflictItem[] = [];
  let progress: TransferProgress | null = null;

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenDrop: UnlistenFn | null = null;

  // ── Lifecycle ──
  onMount(async () => {
    try { cwd = await invoke<string>("remote_home_dir"); } catch { cwd = "/"; }
    try { downloadPath = await invoke<string>("get_download_path"); } catch { downloadPath = ""; }
    await refresh();

    unlistenProgress = await listen<TransferProgress>("transfer:progress", (e) => {
      progress = e.payload;
      if (progress.phase === "done") {
        addToast("Transfer complete", "success");
        setTimeout(() => { progress = null; refresh(); }, 500);
      } else if (progress.phase === "error") {
        addToast(`Transfer failed: ${progress.error ?? ""}`);
        setTimeout(() => (progress = null), 1500);
      }
    });

    // Native OS → app drag-drop (upload)
    unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
      if (!show) return;
      if (event.payload.type === "over") {
        isDragOver = true;
      } else if (event.payload.type === "leave") {
        isDragOver = false;
      } else if (event.payload.type === "drop") {
        isDragOver = false;
        const paths = event.payload.paths;
        if (paths.length > 0) handleNativeDrop(paths);
      }
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenDrop?.();
  });

  // ── Path helpers (remote always uses POSIX) ──
  function join(dir: string, name: string): string {
    return dir.endsWith("/") ? dir + name : dir + "/" + name;
  }
  function parent(dir: string): string {
    if (!dir || dir === "/") return "/";
    const idx = dir.lastIndexOf("/");
    return idx <= 0 ? "/" : dir.slice(0, idx);
  }

  // ── Listing ──
  async function refresh() {
    loading = true;
    try {
      entries = await invoke<FsEntry[]>("list_remote_dir", { path: cwd });
      selected.clear();
      selected = selected;
    } catch (e: any) {
      addToast(`Failed to load list: ${e}`);
      entries = [];
    } finally {
      loading = false;
    }
  }

  async function enter(entry: FsEntry) {
    if (!entry.is_dir) return;
    cwd = join(cwd, entry.name);
    await refresh();
  }
  async function goUp() {
    cwd = parent(cwd);
    await refresh();
  }

  // ── Selection ──
  function selectedItems(): Item[] {
    return Array.from(selected).map((name) => {
      const e = entries.find((x) => x.name === name)!;
      return { path: join(cwd, name), name, is_dir: e?.is_dir ?? false };
    });
  }
  function handleClick(e: MouseEvent, name: string) {
    if (e.shiftKey && lastClickedName) {
      const a = entries.findIndex((x) => x.name === lastClickedName);
      const b = entries.findIndex((x) => x.name === name);
      if (a >= 0 && b >= 0) {
        const [lo, hi] = a < b ? [a, b] : [b, a];
        for (let i = lo; i <= hi; i++) selected.add(entries[i].name);
      }
    } else if (e.ctrlKey || e.metaKey) {
      selected.has(name) ? selected.delete(name) : selected.add(name);
    } else {
      selected.clear();
      selected.add(name);
    }
    selected = selected;
    lastClickedName = name;
  }
  function handleContextMenu(e: MouseEvent, name: string) {
    e.preventDefault();
    if (!selected.has(name)) {
      selected.clear();
      selected.add(name);
      selected = selected;
    }
    contextMenu = { x: e.clientX, y: e.clientY, name };
  }
  function closeContextMenu() { contextMenu = null; }

  // ── Native OS drop (= upload) ──
  async function handleNativeDrop(localPaths: string[]) {
    pendingItems = localPaths.map((p) => ({
      path: p,
      name: p.split(/[\\\/]/).pop() ?? p,
      is_dir: false,
    }));
    pendingTargetDir = cwd;
    pendingDirection = "upload";
    await checkAndRun();
  }

  // ── Download ──
  async function downloadSelected() {
    closeContextMenu();
    const items = selectedItems();
    if (!items.length) return;
    pendingItems = items;
    pendingTargetDir = downloadPath;
    pendingDirection = "download";
    await checkAndRun();
  }

  // ── Transfer pipeline ──
  async function checkAndRun() {
    try {
      const paths = pendingItems.map((i) => i.path);
      conflicts = pendingDirection === "upload"
        ? await invoke<ConflictItem[]>("check_upload_conflicts", { localPaths: paths, remoteDir: pendingTargetDir })
        : await invoke<ConflictItem[]>("check_download_conflicts", { remotePaths: paths, localDir: pendingTargetDir });
    } catch (e: any) {
      addToast(`Conflict check failed: ${e}`);
      conflicts = [];
    }
    if (conflicts.length === 0) {
      await runTransfer("overwrite");
    }
  }

  async function onResolveConflict(e: CustomEvent<{ policy: ConflictPolicy }>) {
    conflicts = [];
    await runTransfer(e.detail.policy);
  }
  function onCancelConflict() {
    conflicts = [];
    pendingItems = [];
    pendingDirection = null;
  }

  async function runTransfer(policy: ConflictPolicy) {
    if (!pendingItems.length || !pendingDirection) return;
    progress = { id: "", phase: "packing", current_bytes: 0, total_bytes: 0,
                 files_done: 0, files_total: 0, current_file: "", error: null };
    try {
      if (pendingDirection === "upload") {
        await invoke("upload_to_remote", {
          localPaths: pendingItems.map((i) => i.path),
          remoteDir: pendingTargetDir,
          policy,
        });
      } else {
        await invoke("download_to_local", {
          remotePaths: pendingItems.map((i) => i.path),
          localDir: pendingTargetDir,
          policy,
        });
      }
    } catch (e: any) {
      progress = { id: "", phase: "error", current_bytes: 0, total_bytes: 0,
                   files_done: 0, files_total: 0, current_file: "", error: String(e) };
      addToast(`Transfer failed: ${e}`);
      setTimeout(() => (progress = null), 2000);
    } finally {
      pendingItems = [];
      pendingDirection = null;
    }
  }

  // ── File ops ──
  function deleteSelected() {
    closeContextMenu();
    const items = selectedItems();
    if (!items.length) return;
    pendingDelete = items;
  }
  async function confirmDelete() {
    const items = pendingDelete;
    pendingDelete = [];
    if (!items.length) return;
    try {
      await invoke("delete_remote_paths", { paths: items.map((i) => i.path) });
      addToast("Deleted", "success");
      await refresh();
    } catch (e: any) {
      addToast(`Delete failed: ${e}`);
    }
  }
  function cancelDelete() { pendingDelete = []; }
  function startRename(name: string) {
    closeContextMenu();
    renamingName = name;
    renameValue = name;
  }
  async function commitRename() {
    if (!renamingName || !renameValue || renameValue === renamingName) {
      renamingName = null;
      return;
    }
    try {
      await invoke("move_remote_path", { src: join(cwd, renamingName), dst: join(cwd, renameValue) });
      addToast("Renamed", "success");
      renamingName = null;
      await refresh();
    } catch (e: any) {
      addToast(`Rename failed: ${e}`);
      renamingName = null;
    }
  }
  async function newFolder() {
    closeContextMenu();
    const name = prompt("New folder name:");
    if (!name) return;
    try {
      await invoke("mkdir_remote_path", { path: join(cwd, name) });
      await refresh();
    } catch (e: any) {
      addToast(`Failed to create folder: ${e}`);
    }
  }

  // ── Settings (download path) ──
  function openSettings() {
    editingDownloadPath = downloadPath;
    showSettings = true;
  }
  async function pickDownloadFolder() {
    const result = await openDialog({ directory: true, defaultPath: editingDownloadPath || undefined });
    if (typeof result === "string") editingDownloadPath = result;
  }
  async function saveSettings() {
    try {
      await invoke("save_download_path", { path: editingDownloadPath });
      downloadPath = await invoke<string>("get_download_path");
      showSettings = false;
      addToast("Saved", "success");
    } catch (e: any) {
      addToast(`Save failed: ${e}`);
    }
  }
  async function resetSettings() {
    editingDownloadPath = "";
    await saveSettings();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) closeExplorer();
  }

  function fmtSize(bytes: number, isDir: boolean): string {
    if (isDir) return "";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }
  function fmtDate(ts: number | null): string {
    if (!ts) return "";
    return new Date(ts * 1000).toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" });
  }
</script>

<svelte:window on:click={closeContextMenu} />

{#if show}
<div class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center"
     on:click={handleBackdropClick} role="dialog">
  <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-[80vw] h-[80vh] max-w-5xl flex flex-col"
       class:ring-2={isDragOver} class:ring-blue-500={isDragOver}>
    <!-- Header -->
    <div class="flex items-center gap-2 px-4 py-3 border-b border-zinc-700">
      <h2 class="text-sm font-semibold flex-1">Server File Explorer</h2>
      <button class="text-xs text-zinc-400 hover:text-white px-2" on:click={openSettings} title="Download path">
        ⚙ Settings
      </button>
      <button class="text-zinc-400 hover:text-white" on:click={closeExplorer}>✕</button>
    </div>

    <!-- Path bar -->
    <div class="bg-zinc-800 px-3 py-2 border-b border-zinc-700 flex items-center gap-2">
      <button class="px-1 text-zinc-400 hover:text-white" on:click={goUp} title="Up">↑</button>
      <input
        class="flex-1 bg-zinc-900 border border-zinc-700 rounded px-2 py-0.5 text-xs"
        bind:value={cwd}
        on:keydown={(e) => e.key === "Enter" && refresh()}
        spellcheck={false}
      />
      <button class="px-2 text-xs text-zinc-400 hover:text-white" on:click={refresh}>⟳</button>
    </div>

    <!-- List -->
    <div class="flex-1 overflow-auto bg-zinc-950">
      {#if loading}
        <div class="p-4 text-zinc-500 text-sm">Loading...</div>
      {:else if entries.length === 0}
        <div class="p-4 text-zinc-500 text-sm">Empty — drag files from your desktop to upload</div>
      {:else}
        <table class="w-full text-xs">
          <thead class="bg-zinc-900 sticky top-0">
            <tr class="text-zinc-500 text-left">
              <th class="px-3 py-1">Name</th>
              <th class="px-3 py-1 w-24">Size</th>
              <th class="px-3 py-1 w-36">Modified</th>
            </tr>
          </thead>
          <tbody>
            {#each entries as entry (entry.name)}
              <tr class="hover:bg-zinc-800 cursor-pointer select-none"
                  class:bg-blue-700={selected.has(entry.name)}
                  on:click={(e) => handleClick(e, entry.name)}
                  on:dblclick={() => enter(entry)}
                  on:contextmenu={(e) => handleContextMenu(e, entry.name)}>
                <td class="px-3 py-1 truncate">
                  <span class="mr-1">{entry.is_dir ? "📁" : "📄"}</span>
                  {#if renamingName === entry.name}
                    <input class="bg-zinc-800 border border-blue-500 px-1 text-xs"
                           bind:value={renameValue}
                           on:blur={commitRename}
                           on:keydown={(e) => {
                             if (e.key === "Enter") commitRename();
                             if (e.key === "Escape") renamingName = null;
                           }}
                           autofocus />
                  {:else}
                    {entry.name}
                  {/if}
                </td>
                <td class="px-3 py-1 text-zinc-500">{fmtSize(entry.size, entry.is_dir)}</td>
                <td class="px-3 py-1 text-zinc-500">{fmtDate(entry.modified)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-4 py-2 border-t border-zinc-700 text-xs text-zinc-500 flex items-center gap-3">
      <span>Drop to upload · Right-click menu · Double-click to enter folder</span>
      <span class="ml-auto">↓ {downloadPath || "(unset)"}</span>
    </div>
  </div>
</div>
{/if}

<!-- Context menu -->
{#if contextMenu}
  {@const menuName = contextMenu.name}
  <div class="fixed z-[100] bg-zinc-800 border border-zinc-600 rounded shadow-lg text-xs min-w-[160px]"
       style="left: {contextMenu.x}px; top: {contextMenu.y}px"
       on:click|stopPropagation role="menu">
    <button class="w-full text-left px-3 py-1.5 hover:bg-zinc-700" on:click={downloadSelected}>
      ⬇ Download ({selected.size})
    </button>
    <div class="border-t border-zinc-600"></div>
    <button class="w-full text-left px-3 py-1.5 hover:bg-zinc-700" on:click={() => startRename(menuName)}>
      Rename
    </button>
    <button class="w-full text-left px-3 py-1.5 hover:bg-zinc-700" on:click={newFolder}>
      New folder
    </button>
    <div class="border-t border-zinc-600"></div>
    <button class="w-full text-left px-3 py-1.5 hover:bg-zinc-700 text-red-400" on:click={deleteSelected}>
      Delete
    </button>
  </div>
{/if}

<!-- Settings popup (closes only via Cancel/Save buttons, not backdrop) -->
{#if showSettings}
<div class="fixed inset-0 bg-black/70 z-[60] flex items-center justify-center" role="dialog">
  <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-[480px] p-5">
    <h3 class="text-sm font-semibold mb-3">Download path</h3>
    <p class="text-xs text-zinc-400 mb-2">Local folder where downloaded files will be saved.</p>
    <div class="flex gap-2 mb-3">
      <input class="flex-1 bg-zinc-800 border border-zinc-700 rounded px-2 py-1 text-xs"
             bind:value={editingDownloadPath}
             placeholder="(empty = ~/Downloads)"
             spellcheck={false} />
      <button class="px-3 py-1 bg-zinc-700 hover:bg-zinc-600 rounded text-xs" on:click={pickDownloadFolder}>
        Pick folder
      </button>
    </div>
    <div class="flex gap-2 justify-end">
      <button class="px-3 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-xs" on:click={resetSettings}>
        Reset to default
      </button>
      <button class="px-3 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-xs" on:click={() => (showSettings = false)}>
        Cancel
      </button>
      <button class="px-3 py-1 bg-blue-700 hover:bg-blue-600 rounded text-xs" on:click={saveSettings}>
        Save
      </button>
    </div>
  </div>
</div>
{/if}

{#if conflicts.length > 0}
  <ConflictModal {conflicts} on:resolve={onResolveConflict} on:cancel={onCancelConflict} />
{/if}

{#if pendingDelete.length > 0}
  <ConfirmModal
    title="Delete"
    message={`Delete ${pendingDelete.length} item(s)?\n\n${pendingDelete.map((i) => i.name).join("\n")}`}
    confirmLabel="Delete"
    cancelLabel="Cancel"
    danger
    on:confirm={confirmDelete}
    on:cancel={cancelDelete}
  />
{/if}

<ProgressModal {progress} />
