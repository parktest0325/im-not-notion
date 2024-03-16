<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import { selectedFilePath } from "./stores.js";
    import { invoke } from "@tauri-apps/api";
    import { v4 as uuidv4 } from "uuid";

    let fileContent: string = "";
    let editable: boolean = false;
    let showDialog: boolean = false; // 대화상자 표시 상태를 위한 일반 boolean 변수
    let contentTextArea: HTMLTextAreaElement;

    $: if ($selectedFilePath) {
        getFileContent($selectedFilePath);
    }

    $: if (editable) {
        contentTextArea?.focus();
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

    // TreeNode에 이름 변경을 위해 키다운리스너가 전역적으로 등록되어 있는데,
    // Enter 키가 TextArea에서 사용하는 개행과 겹쳐 발생하는 문제 해결을 위해
    // window나 document에 등록한 이벤트는 가장 늦게 처리되기 때문에
    // 이 이벤트는 TextArea에 직접 등록해 먼저 처리하고, stopPropagation으로 버블링을 막는다.
    // ctrl+s, Escape 일땐 기본 동작을 제거하고, 특정 동작을 수행하고 나머지 키는 기본동작대로 동작시킨다.
    function handleKeyDown(event: KeyboardEvent) {
        console.log("onKeyDown");
        if (editable) {
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

    // 붙여넣기 이벤트 핸들러
    async function handlePaste(event: ClipboardEvent) {
        const items = event.clipboardData?.items;

        if (items) {
            const item = items[0]; // 클립보드의 맨 위 아이템만 검사

            // 클립보드 내용이 이미지인 경우
            if (item.type.indexOf("image") !== -1) {
                event.preventDefault(); // 기본 붙여넣기 동작을 방지

                try {
                    const fileData = await readFileAsArrayBuffer(
                        item.getAsFile()!,
                    );

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

                    // 이미지 태그를 현재 커서 위치에 추가
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
                // reader.result는 ArrayBuffer | null 타입을 가집니다.
                // TypeScript의 타입 가드를 사용하여 ArrayBuffer인 경우만 처리합니다.
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
    <!-- 대화상자 컨텐츠 -->
    <div
        class="fixed inset-0 bg-gray-600 bg-opacity-50 flex justify-center items-center"
    >
        <div class="bg-white p-4 rounded-lg shadow-lg space-y-4">
            <p>Are you Saving?</p>
            <div class="flex justify-end space-x-2">
                <button
                    class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-700"
                    on:click={() => {
                        saveContent();
                        showDialog = false;
                    }}>Yes</button
                >
                <button
                    class="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-700"
                    on:click={() => {
                        showDialog = false;
                        editable = true;
                    }}>No</button
                >
            </div>
        </div>
    </div>
{/if}
<div class="main-content overflow-y-auto h-full w-full">
    {#if editable}
        <textarea
            bind:this={contentTextArea}
            class="whitespace-pre-wrap w-full h-full resize-none"
            bind:value={fileContent}
            on:keydown={handleKeyDown}
            on:blur={() => (editable = false)}
            on:paste={handlePaste}
        ></textarea>
    {:else}
        <div
            tabindex="0"
            role="button"
            class="break-all w-full h-full whitespace-pre-wrap"
            on:dblclick={() => {
                if ($selectedFilePath != "") {
                    editable = true;
                }
            }}
        >
            {fileContent}
        </div>
    {/if}
</div>
