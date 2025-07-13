<script lang="ts">
  import { fullFilePath, isConnected, relativeFilePath } from "../stores";
  import { invoke } from "@tauri-apps/api/core";
  import { v4 as uuidv4 } from "uuid";
  import { tick, onMount, onDestroy } from "svelte";
  import { writable } from "svelte/store";
  import SavePopup from "./SavePopup.svelte";

  let fileContent: string = "";
  let editable: boolean = false;
  let showDialog: boolean = false; // 대화상자 표시 상태를 위한 일반 boolean 변수
  let contentTextArea: HTMLTextAreaElement;
  let contentDiv: HTMLDivElement;
  let scrollPosition: number = 0; // 스크롤 위치를 저장할 변수 추가

  let isContentChanged: boolean = false; // 내용 변경 여부를 추적하는 변수
  let autoSaveEnabled = writable(true); // 자동 저장 기능 활성화 여부
  let autoSaveInterval = 1000 * 5; //* 60; // 자동 저장 주기 (5분)
  let autoSaveTimer: number | null = null; // 자동 저장 타이머

  $: if ($fullFilePath) {
    getFileContent($fullFilePath);
    scrollPosition = 0;
    contentDiv?.scrollTo(0, 0);
    editable = false;
  }

  $: if (editable) {
    tick().then(() => {
      contentTextArea?.focus();
      contentTextArea?.scrollTo(0, scrollPosition);
      startAutoSave(); // editable이 true일 때 자동 저장 시작
    });
  } else {
    tick().then(() => {
      contentDiv?.scrollTo(0, scrollPosition);
      stopAutoSave(); // editable이 false일 때 자동 저장 중지
    });
  }

  async function getFileContent(filePath: string) {
    try {
      const content: string = await invoke("get_file_content", {
        filePath,
      });
      console.log("content: ", content);
      fileContent = content;
      isContentChanged = false; // 파일 내용을 불러올 때 변경 여부 초기화
      // 성공적으로 파일 내용을 불러온 경우 연결 상태를 갱신
      isConnected.set(true);
      console.log("filecontent: ", fileContent);
    } catch (error) {
      console.error("Failed to get file content", error);
      fileContent = "파일을 불러오는데 실패했습니다.";
      isConnected.set(false);
    }
  }

  function startAutoSave() {
    if ($autoSaveEnabled && !autoSaveTimer) {
      autoSaveTimer = setInterval(() => {
        saveContent();
      }, autoSaveInterval);
      console.log(autoSaveTimer);
    }
  }

  function stopAutoSave() {
    if (autoSaveTimer) {
      clearInterval(autoSaveTimer);
      autoSaveTimer = null;
    }
  }

  async function saveContent() {
    if (!isContentChanged) {
      return;
    }
    try {
      await invoke("save_file_content", {
        filePath: $fullFilePath,
        fileData: fileContent,
      });
      console.log("저장되었습니다.");
      isContentChanged = false; // 저장 후 내용 변경 여부를 false로 설정
      isConnected.set(true);
    } catch (error) {
      console.log("저장에 실패했습니다. reason => ", error);
      isConnected.set(false);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    console.log("onKeyDown");
    if (editable) {
      scrollPosition = contentTextArea.scrollTop;
      event.stopPropagation();
      if ((event.ctrlKey || event.metaKey) && event.key === "s") {
        event.preventDefault();
        console.log("ctrl+ s");
        showDialog = true;
        // editable = false;
      } else if (event.key === "Escape") {
        event.preventDefault();
        editable = false;
      } else {
        isContentChanged = true; // 키 입력 시 내용 변경 여부를 true로 설정
      }
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
          console.log("uuid: ", uuidValue);
          const savedPath = await invoke("save_file_image", {
            filePath: $relativeFilePath,
            fileName: uuidValue,
            fileData: Array.from(fileData),
          });
          console.log("savedPath: ", savedPath);

          fileContent = `${beforeText}\n![${uuidValue}](${savedPath})${afterText}`;
        } catch (e) {
          console.log("error: ", e);
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
  });
</script>

<SavePopup show={showDialog}
  closeSave={() => {
    showDialog = false;
    editable = true;
  }}
  handleSave={() => {
    saveContent();
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
        if ($fullFilePath != "") {
          scrollPosition = contentDiv.scrollTop;
          editable = true;
        }
      }}
    >
      {fileContent}
    </div>
  {/if}
</div>
