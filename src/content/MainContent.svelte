<script lang="ts">
  import { isConnected, relativeFilePath, isEditingContent, addToast } from "../stores";
  import { invoke } from "@tauri-apps/api/core";
  import { v4 as uuidv4 } from "uuid";
  import { tick, onMount, onDestroy } from "svelte";
  import { writable } from "svelte/store";
  import SavePopup from "./SavePopup.svelte";
  import { registerAction, unregisterAction } from "../shortcut";

  let fileContent: string = "";
  let editable: boolean = false;
  let showDialog: boolean = false;
  let contentTextArea: HTMLTextAreaElement;
  let contentDiv: HTMLDivElement;
  let scrollPosition: number = 0;

  let isContentChanged: boolean = false;
  let autoSaveEnabled = writable(true);
  let autoSaveInterval = 1000 * 5;
  let autoSaveTimer: number | null = null;

  $: if ($relativeFilePath) {
    getFileContent($relativeFilePath);
    scrollPosition = 0;
    contentDiv?.scrollTo(0, 0);
    editable = false;
  }

  $: if (editable) {
    isEditingContent.set(true);
    tick().then(() => {
      contentTextArea?.focus();
      contentTextArea?.scrollTo(0, scrollPosition);
      startAutoSave();
    });
  } else {
    isEditingContent.set(false);
    tick().then(() => {
      contentDiv?.scrollTo(0, scrollPosition);
      stopAutoSave();
    });
  }

  // Register shortcut actions
  onMount(() => {
    registerAction("save", () => {
      if (editable) showDialog = true;
    });
    registerAction("exit-edit", () => {
      if (editable) editable = false;
    });
  });

  async function getFileContent(filePath: string) {
    try {
      const content: string = await invoke("get_file_content", {
        filePath,
      });
      fileContent = content;
      isContentChanged = false;
      isConnected.set(true);
    } catch (error) {
      console.error("Failed to get file content", error);
      fileContent = "Failed to load file.";
      isConnected.set(false);
    }
  }

  function startAutoSave() {
    if ($autoSaveEnabled && !autoSaveTimer) {
      autoSaveTimer = setInterval(() => {
        saveContent();
      }, autoSaveInterval);
    }
  }

  function stopAutoSave() {
    if (autoSaveTimer) {
      clearInterval(autoSaveTimer);
      autoSaveTimer = null;
    }
  }

  async function saveContent(manual: boolean = false) {
    if (!isContentChanged) {
      return;
    }
    try {
      await invoke("save_file_content", {
        filePath: $relativeFilePath,
        fileData: fileContent,
        manual,
      });
      isContentChanged = false;
      isConnected.set(true);
    } catch (error) {
      console.error("Failed to save content:", error);
      isConnected.set(false);
      addToast("Failed to save file.");
    }
  }

  function handleKeyDown(_event: KeyboardEvent) {
    if (editable) {
      scrollPosition = contentTextArea.scrollTop;
      isContentChanged = true;
    }
  }

  async function handlePaste(event: ClipboardEvent) {
    const items = event.clipboardData?.items;

    if (items) {
      const item = items[0];

      if (item.type.indexOf("image") !== -1) {
        event.preventDefault();

        try {
          const fileData = await readFileAsArrayBuffer(item.getAsFile()!);

          const textarea = event.target as HTMLTextAreaElement;
          const currentPosition = textarea.selectionStart || 0;
          const beforeText = fileContent.slice(0, currentPosition);
          const afterText = fileContent.slice(currentPosition);

          const uuidValue = uuidv4();
          const savedPath = await invoke("save_file_image", {
            filePath: $relativeFilePath,
            fileName: uuidValue,
            fileData: Array.from(fileData),
          });

          fileContent = `${beforeText}\n![${uuidValue}](${savedPath})${afterText}`;
        } catch (e) {
          console.error("Image paste failed:", e);
        addToast("Failed to save image.");
        }
      }
    }
  }

  async function readFileAsArrayBuffer(file: File): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        if (reader.result instanceof ArrayBuffer) {
          resolve(new Uint8Array(reader.result));
        } else {
          reject(new Error("File reading resulted in null"));
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsArrayBuffer(file);
    });
  }

  onDestroy(() => {
    stopAutoSave();
    unregisterAction("save");
    unregisterAction("exit-edit");
  });
</script>

<SavePopup show={showDialog}
  closeSave={() => {
    showDialog = false;
    editable = true;
  }}
  handleSave={async () => {
    await saveContent(true);
    addToast("File saved.", "success");
    editable = false;
    showDialog = false;
  }}
/>

<div bind:this={contentDiv} class="{editable ? 'overflow-hidden' : 'overflow-y-auto'} h-full w-full">
  {#if editable}
    <textarea
      on:paste={handlePaste}
      bind:this={contentTextArea}
      class="whitespace-pre-wrap resize-none p-4 w-full h-full"
      bind:value={fileContent}
      on:keydown={handleKeyDown}
    ></textarea>
  {:else}
    <div
      tabindex="0"
      role="button"
      class="break-all w-full h-full whitespace-pre-wrap p-4"
      on:dblclick={() => {
        if ($relativeFilePath != "") {
          scrollPosition = contentDiv.scrollTop;
          editable = true;
        }
      }}
    >
      {fileContent}
    </div>
  {/if}
</div>
