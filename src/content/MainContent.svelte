<script lang="ts">
  import { isConnected, relativeFilePath, selectedCursor, isEditingContent, addToast } from "../stores";
  import { invoke } from "@tauri-apps/api/core";
  import { v4 as uuidv4 } from "uuid";
  import { tick, onMount, onDestroy } from "svelte";
  import { writable, get } from "svelte/store";
  import SavePopup from "./SavePopup.svelte";
  import { registerAction, unregisterAction } from "../shortcut";

  // CodeMirror
  import { EditorView, keymap } from "@codemirror/view";
  import { EditorState, Compartment } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { search, searchKeymap } from "@codemirror/search";
  import { markdown } from "@codemirror/lang-markdown";

  let fileContent: string = "";
  let editable: boolean = false;
  let showDialog: boolean = false;
  let contentDiv: HTMLDivElement;
  let editorContainer: HTMLDivElement;
  let scrollRatio: number = 0;

  let isContentChanged: boolean = false;
  let autoSaveEnabled = writable(true);
  let autoSaveInterval = 1000 * 5;
  let autoSaveTimer: number | null = null;

  // Unsaved changes dialog
  let currentFilePath: string = "";
  let showUnsavedDialog: boolean = false;
  let unsavedAction: "exit" | "switch" | null = null;
  let unsavedSwitchPath: string | null = null;
  let unsavedSwitchCursor: string | null = null;

  // CodeMirror
  let view: EditorView | null = null;
  const editableCompartment = new Compartment();

  // 앱 CSS 변수 기반 테마
  const innTheme = EditorView.theme({
    "&": {
      backgroundColor: "var(--content-bg-color)",
      color: "var(--reverse-primary-color)",
      height: "100%",
      fontSize: "inherit",
      border: "2px solid var(--hover-border-color)",
      borderRadius: "8px",
      boxShadow: "0 2px 2px rgba(0, 0, 0, 0.2)",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-scroller": {
      fontFamily: "inherit",
      lineHeight: "inherit",
      overflow: "auto",
    },
    ".cm-content": {
      padding: "1rem",
      whiteSpace: "pre-wrap",
      wordBreak: "break-all",
      caretColor: "var(--reverse-primary-color)",
    },
    ".cm-line": {
      padding: "0",
    },
    ".cm-gutters": {
      display: "none",
    },
    ".cm-cursor": {
      borderLeftColor: "var(--reverse-primary-color)",
    },
    ".cm-selectionBackground": {
      backgroundColor: "rgba(100, 149, 237, 0.3) !important",
    },
    "&.cm-focused .cm-selectionBackground": {
      backgroundColor: "rgba(100, 149, 237, 0.4) !important",
    },
    // 검색 하이라이트
    ".cm-searchMatch": {
      backgroundColor: "var(--search-match-bg)",
      borderRadius: "2px",
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
      backgroundColor: "var(--search-match-current-bg)",
    },
    // 검색 패널
    ".cm-panel.cm-search": {
      backgroundColor: "var(--search-bar-bg)",
      borderBottom: "1px solid var(--search-bar-border)",
      padding: "4px 8px",
    },
    ".cm-panel.cm-search input": {
      backgroundColor: "var(--input-bg-color)",
      color: "var(--reverse-primary-color)",
      border: "1px solid var(--border-color)",
      borderRadius: "3px",
      padding: "2px 6px",
      fontSize: "0.8rem",
      outline: "none",
    },
    ".cm-panel.cm-search input:focus": {
      borderColor: "var(--highlight-color)",
    },
    ".cm-panel.cm-search button": {
      backgroundColor: "transparent",
      color: "var(--reverse-primary-color)",
      border: "1px solid var(--border-color)",
      borderRadius: "3px",
      padding: "2px 8px",
      fontSize: "0.75rem",
      cursor: "pointer",
    },
    ".cm-panel.cm-search button:hover": {
      backgroundColor: "var(--button-hover-bg-color)",
    },
    ".cm-panel.cm-search label": {
      color: "var(--reverse-secondary-color)",
      fontSize: "0.75rem",
    },
    ".cm-panel.cm-search .cm-button": {
      backgroundImage: "none",
    },
  });

  function createEditorView() {
    if (view) {
      view.destroy();
      view = null;
    }
    if (!editorContainer) return;

    const state = EditorState.create({
      doc: fileContent,
      extensions: [
        editableCompartment.of(EditorView.editable.of(editable)),
        innTheme,
        history(),
        markdown(),
        search(),
        keymap.of([
          // Ctrl+S / Cmd+S → 저장
          { key: "Mod-s", run: () => { if (editable) showDialog = true; return true; } },
          // Escape → exit edit (검색 패널이 없을 때)
          { key: "Escape", run: (v) => {
            if (editable) { tryExitEdit(); return true; }
            return false;
          }},
          ...searchKeymap,
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        // 문서 변경 감지
        EditorView.updateListener.of(update => {
          if (update.docChanged) {
            fileContent = update.state.doc.toString();
            isContentChanged = true;
          }
        }),
        // 이미지 붙여넣기
        EditorView.domEventHandlers({
          paste: handlePaste,
        }),
        // 줄바꿈
        EditorView.lineWrapping,
      ],
    });

    view = new EditorView({
      state,
      parent: editorContainer,
    });
  }

  // --- Edit mode transitions ---
  // NOTE: view, fileContent, scrollRatio must NOT appear directly
  //       in `$:` blocks — Svelte tracks them as reactive dependencies
  //       and would cause infinite re-runs when createEditorView sets `view`.

  function enterEditMode() {
    isEditingContent.set(true);
    tick().then(() => {
      createEditorView();
      tick().then(() => {
        if (view) {
          view.focus();
          requestAnimationFrame(() => {
            if (view) {
              const max = view.scrollDOM.scrollHeight - view.scrollDOM.clientHeight;
              view.scrollDOM.scrollTop = max > 0 ? scrollRatio * max : 0;
            }
          });
        }
      });
      startAutoSave();
    });
  }

  function exitEditMode() {
    isEditingContent.set(false);
    if (view) {
      fileContent = view.state.doc.toString();
      const max = view.scrollDOM.scrollHeight - view.scrollDOM.clientHeight;
      scrollRatio = max > 0 ? view.scrollDOM.scrollTop / max : 0;
      view.destroy();
      view = null;
    }
    tick().then(() => {
      if (contentDiv) {
        const max = contentDiv.scrollHeight - contentDiv.clientHeight;
        contentDiv.scrollTo(0, max > 0 ? scrollRatio * max : 0);
      }
      stopAutoSave();
    });
  }

  // --- Unsaved changes ---

  function tryExitEdit() {
    if (isContentChanged) {
      stopAutoSave();
      unsavedAction = "exit";
      showUnsavedDialog = true;
    } else {
      editable = false;
    }
  }

  function handleFilePathChange(newPath: string) {
    if (!newPath || newPath === currentFilePath) return;

    if (editable && isContentChanged) {
      stopAutoSave();
      unsavedAction = "switch";
      unsavedSwitchPath = newPath;
      unsavedSwitchCursor = get(selectedCursor);
      // 즉시 되돌려서 TopBar/사이드바가 점프하지 않도록
      relativeFilePath.set(currentFilePath);
      selectedCursor.set(currentFilePath);
      showUnsavedDialog = true;
    } else {
      switchToFile(newPath);
    }
  }

  function switchToFile(path: string, cursor?: string) {
    currentFilePath = path;
    relativeFilePath.set(path);
    if (cursor) selectedCursor.set(cursor);
    getFileContent(path);
    scrollRatio = 0;
    contentDiv?.scrollTo(0, 0);
    editable = false;
  }

  async function handleUnsavedSave() {
    // 항상 원래 파일(currentFilePath)에 저장
    await saveContent(true, currentFilePath);

    // 저장 완전 실패 시 (SSH 끊김 등) 편집 모드 유지
    if (isContentChanged) {
      showUnsavedDialog = false;
      unsavedAction = null;
      unsavedSwitchPath = null;
      unsavedSwitchCursor = null;
      startAutoSave();
      return;
    }

    showUnsavedDialog = false;
    addToast("File saved.", "success");

    if (unsavedAction === "exit") {
      await getFileContent(currentFilePath);
      editable = false;
    } else if (unsavedAction === "switch" && unsavedSwitchPath) {
      switchToFile(unsavedSwitchPath, unsavedSwitchCursor ?? undefined);
    }
    unsavedAction = null;
    unsavedSwitchPath = null;
    unsavedSwitchCursor = null;
  }

  async function handleUnsavedDiscard() {
    showUnsavedDialog = false;
    isContentChanged = false;

    if (unsavedAction === "exit") {
      await getFileContent(currentFilePath);
      editable = false;
    } else if (unsavedAction === "switch" && unsavedSwitchPath) {
      switchToFile(unsavedSwitchPath, unsavedSwitchCursor ?? undefined);
    }
    unsavedAction = null;
    unsavedSwitchPath = null;
    unsavedSwitchCursor = null;
  }

  function handleUnsavedCancel() {
    showUnsavedDialog = false;
    // stores는 handleFilePathChange에서 이미 되돌림
    unsavedAction = null;
    unsavedSwitchPath = null;
    unsavedSwitchCursor = null;
    startAutoSave();
  }

  // --- Reactive ---

  $: if ($relativeFilePath) {
    handleFilePathChange($relativeFilePath);
  }

  $: if (editable) {
    enterEditMode();
  } else {
    exitEditMode();
  }

  // fileContent가 외부에서 바뀌었을 때 (getFileContent 호출 등) 에디터에 반영
  function syncEditorContent() {
    if (view && view.state.doc.toString() !== fileContent) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: fileContent },
      });
    }
  }

  // --- Shortcuts ---

  onMount(() => {
    registerAction("save", () => {
      if (editable) showDialog = true;
    });
    registerAction("exit-edit", () => {
      if (editable) tryExitEdit();
    });
  });

  // --- File operations ---

  async function getFileContent(filePath: string) {
    try {
      const content: string = await invoke("get_file_content", {
        filePath,
      });
      fileContent = content;
      isContentChanged = false;
      isConnected.set(true);
      syncEditorContent();
    } catch (error) {
      console.error("Failed to get file content", error);
      fileContent = "";
      syncEditorContent();
      const connected: boolean = await invoke("check_connection");
      isConnected.set(connected);
      if (!connected) {
        addToast("SSH connection lost.");
      } else {
        addToast("Failed to load file.");
      }
    }
  }

  function startAutoSave() {
    if ($autoSaveEnabled && !autoSaveTimer) {
      autoSaveTimer = setInterval(() => {
        saveContent(false, currentFilePath);
      }, autoSaveInterval);
    }
  }

  function stopAutoSave() {
    if (autoSaveTimer) {
      clearInterval(autoSaveTimer);
      autoSaveTimer = null;
    }
  }

  async function saveContent(manual: boolean = false, savePath?: string): Promise<boolean> {
    if (!isContentChanged) {
      return true;
    }
    // 에디터에서 최신 내용 가져오기
    if (view) {
      fileContent = view.state.doc.toString();
    }
    try {
      const syncOk = await invoke<boolean>("save_file_content", {
        filePath: savePath ?? $relativeFilePath,
        fileData: fileContent,
        manual,
      });
      isContentChanged = false;
      isConnected.set(true);
      return syncOk;
    } catch (error) {
      console.error("Failed to save content:", error);
      const connected: boolean = await invoke("check_connection");
      isConnected.set(connected);
      if (!connected) {
        addToast("SSH connection lost.");
      } else {
        addToast("Failed to save file.");
      }
      return false;
    }
  }

  // --- Image paste ---

  async function handlePaste(event: ClipboardEvent, cmView: EditorView): boolean {
    const items = event.clipboardData?.items;

    if (items) {
      const item = items[0];

      if (item.type.indexOf("image") !== -1) {
        event.preventDefault();

        try {
          const fileData = await readFileAsArrayBuffer(item.getAsFile()!);
          const currentPosition = cmView.state.selection.main.head;

          const uuidValue = uuidv4();
          const savedPath = await invoke("save_file_image", {
            filePath: currentFilePath,
            fileName: uuidValue,
            fileData: Array.from(fileData),
          });

          const insertText = `\n![${uuidValue}](${savedPath})`;
          cmView.dispatch({
            changes: { from: currentPosition, insert: insertText },
          });
          isContentChanged = true;
        } catch (e) {
          console.error("Image paste failed:", e);
          addToast("Failed to save image.");
        }
        return true;
      }
    }
    return false;
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
    if (view) { view.destroy(); view = null; }
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
    const syncOk = await saveContent(true, currentFilePath);
    showDialog = false;
    if (syncOk) {
      await getFileContent(currentFilePath);
      addToast("File saved.", "success");
      editable = false;
    } else {
      addToast("File saved, but image sync failed.", "warning");
    }
  }}
/>

<SavePopup show={showUnsavedDialog}
  title="Unsaved Changes"
  message="You have unsaved changes."
  saveLabel="Save"
  discardLabel="Discard"
  cancelLabel="Cancel"
  closeSave={handleUnsavedCancel}
  handleSave={handleUnsavedSave}
  handleDiscard={handleUnsavedDiscard}
/>

<div bind:this={contentDiv} class="{editable ? 'overflow-hidden' : 'overflow-y-auto'} h-full w-full">
  {#if editable}
    <div bind:this={editorContainer} class="h-full w-full"></div>
  {:else}
    <div
      tabindex="0"
      role="button"
      class="break-all w-full min-h-full whitespace-pre-wrap p-4"
      style="border: 2px solid transparent; border-radius: 8px;"
      on:dblclick={() => {
        if ($relativeFilePath != "") {
          const max = contentDiv.scrollHeight - contentDiv.clientHeight;
          scrollRatio = max > 0 ? contentDiv.scrollTop / max : 0;
          editable = true;
        }
      }}
    >
      {fileContent + '\n'}
    </div>
  {/if}
</div>
