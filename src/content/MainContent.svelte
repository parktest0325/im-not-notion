<script lang="ts">
    import { selectedFilePath } from "../stores.js";
    import { invoke } from "@tauri-apps/api";
    import { v4 as uuidv4 } from "uuid";
    import { tick } from "svelte";
  
    let fileContent: string = "";
    let editable: boolean = false;
    let showDialog: boolean = false; // 대화상자 표시 상태를 위한 일반 boolean 변수
    let contentTextArea: HTMLTextAreaElement;
    let contentDiv: HTMLDivElement;
    let scrollPosition: number = 0; // 스크롤 위치를 저장할 변수 추가
  
    $: if ($selectedFilePath) {
      getFileContent($selectedFilePath);
      scrollPosition = 0;
      contentDiv?.scrollTo(0, 0);
      editable = false;
    }
  
    $: if (editable) {
      tick().then(() => {
        contentTextArea?.focus();
        contentTextArea?.scrollTo(0, scrollPosition);
      });
    }
  
    $: if (!editable) {
      tick().then(() => {
        contentDiv?.scrollTo(0, scrollPosition);
      });
    }
  
    async function getFileContent(filePath: string) {
      try {
        const content: string = await invoke("get_file_content", {
          filePath,
        });
        console.log("content: ", content);
        fileContent = content;
        console.log("filecontent: ", fileContent);
      } catch (error) {
        console.error("Failed to get file content", error);
        fileContent = "파일을 불러오는데 실패했습니다.";
      }
    }
  
    async function saveContent() {
      try {
        await invoke("save_file_content", {
          filePath: $selectedFilePath,
          fileData: fileContent,
        });
        console.log("저장되었습니다.");
        editable = false;
      } catch (error) {
        console.log("저장에 실패했습니다. reason => ", error);
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
          editable = false;
        } else if (event.key === "Escape") {
          event.preventDefault();
          editable = false;
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
              filePath: $selectedFilePath,
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
  </script>
  
  {#if showDialog}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 flex justify-center items-center">
      <div class="bg-white p-4 rounded-lg shadow-lg space-y-4">
        <p>Are you Saving?</p>
        <div class="flex justify-end space-x-2">
          <button
            class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-700"
            on:click={() => {
              saveContent();
              showDialog = false;
            }}>Yes</button>
          <button
            class="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-700"
            on:click={() => {
              showDialog = false;
              editable = true;
            }}>No</button>
        </div>
      </div>
    </div>
  {/if}
  <div bind:this={contentDiv} class="{editable ? 'overflow-hidden' : 'overflow-y-auto'} h-full w-full" style="background-color: var(--maincontent-bg-color);">
    {#if editable}
      <textarea
        on:paste={handlePaste}
        bind:this={contentTextArea}
        class="editable-textarea whitespace-pre-wrap resize-none p-4 w-full h-full"
        bind:value={fileContent}
        on:keydown={handleKeyDown}
      ></textarea>
    {:else}
      <div
        tabindex="0"
        role="button"
        class="break-all w-full h-full whitespace-pre-wrap p-4"
        on:dblclick={() => {
          if ($selectedFilePath != "") {
            scrollPosition = contentDiv.scrollTop;
            editable = true;
          }
        }}
      >
        {fileContent}
      </div>
    {/if}
  </div>
  
  <style>
    .editable-textarea {
      background-color: var(--maincontent-bg-color);
      color: var(--text-color);
      border: 2px solid var(--input-button-border-hover-color);
      border-radius: 8px;
      box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
    }
  </style>
  