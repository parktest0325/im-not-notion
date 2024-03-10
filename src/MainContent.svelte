<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import { selectedFilePath } from "./stores.js";
    import { invoke } from "@tauri-apps/api";

    let fileContent: string = "";
    let editable: boolean = false;
    let showDialog: boolean = false; // 대화상자 표시 상태를 위한 일반 boolean 변수

    $: if ($selectedFilePath) {
        getFileContent($selectedFilePath);
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
            // await invoke("save_content", {
            //     filePath: $selectedFilePath,
            //     content: fileContent,
            // });
            console.log("저장되었습니다.");
            editable = false;
        } catch (error) {
            console.log("저장에 실패했습니다.");
        }
    }

    function handleKeyDown(event: KeyboardEvent) {
        console.log("onKeyDown");
        if (editable && event.ctrlKey && event.key === "s") {
            event.preventDefault();
            console.log("ctrl+ s");
            showDialog = true;
            editable = false;
        }
    }

    onMount(() => {
        window.addEventListener("keydown", handleKeyDown);
    });

    onDestroy(() => {
        window.removeEventListener("keydown", handleKeyDown);
    });

    function handleInput(e: Event) {
        const target = e.target as HTMLElement; // HTMLElement로 타입 단언
        console.log("innerText: " + target.innerText);
        fileContent = target.innerText; // 'innerText' 속성에 안전하게 접근
        console.log("fileContent: " + fileContent);
    }
</script>

{#if showDialog}
    <!-- 대화상자 컨텐츠 -->
    <div
        class="fixed inset-0 bg-gray-600 bg-opacity-50 flex justify-center items-center"
    >
        <div class="bg-white p-4 rounded-lg shadow-lg space-y-4">
            <p>저장하시겠습니까?</p>
            <div class="flex justify-end space-x-2">
                <button
                    class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-700"
                    on:click={() => {
                        saveContent();
                        showDialog = false;
                    }}>예</button
                >
                <button
                    class="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-700"
                    on:click={() => {
                        showDialog = false;
                        editable = true;
                    }}>아니오</button
                >
            </div>
        </div>
    </div>
{/if}
<div class="main-content overflow-y-auto h-full w-full">
    {#if editable}
        <textarea
            class="whitespace-pre-wrap w-full h-full resize-none"
            bind:value={fileContent}
            on:blur={() => (editable = false)}
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
