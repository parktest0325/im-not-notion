<script context="module" lang="ts">
    import { invoke } from "@tauri-apps/api/core";

    let directoryStructure = writable<FileSystemNode[]>([]);
    export async function refreshList() {
        try {
            const data: FileSystemNode = await invoke("get_file_list_");
            directoryStructure.set(data.children);
            isConnected.set(true); // 파일 리스트를 정상적으로 가져온 경우
            console.log(data);
        } catch (error) {
            console.error("Failed to update file list:", error);
            isConnected.set(false); // 파일 리스트를 가져오지 못한 경우
        }
    }
</script>

<script lang="ts">
    import FaSearch from "svelte-icons/fa/FaSearch.svelte";
    import IoMdRefresh from "svelte-icons/io/IoMdRefresh.svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { setContext, onMount } from "svelte";
    import { selectedCursor, relativeFilePath, GLOBAL_FUNCTIONS, isConnected } from "../stores";
    import type { FileSystemNode } from "../types/setting";

    let searchTerm: string = "";

    onMount(refreshList);

    const searchFiles = (_term: string) => {
        // TODO: 검색 로직 구현
    };

    async function createFolder(event: MouseEvent) {
        event.stopPropagation();
        try {
            const createdPath: string = await invoke("new_content_for_hugo", {
                filePath: "/new_folder/_index.md",
            });
            selectedCursor.set(createdPath);
            relativeFilePath.set(createdPath);
            await refreshList();
        } catch (error) {
            console.error("failed to make directory:", error);
        }
    }
</script>

<div class="flex flex-col h-full">
    <!-- 검색 영역 -->
    <div class="flex space-x-2 h-6 mb-4" style="flex-wrap: nowrap;">
        <input
            type="text"
            placeholder="Search..."
            class="flex-grow p-2 border rounded"
            bind:value={searchTerm}
            on:input={() => searchFiles(searchTerm)}
            style="min-width: 0; width: auto; flex-grow: 1;"
        />
        <button>
            <div class="w-5 h-5">
                <FaSearch />
            </div>
        </button>
        <button on:click={refreshList}>
            <div class="w-5 h-5">
                <IoMdRefresh />
            </div>
        </button>
    </div>

    <!-- 파일 리스트 -->
    <div class="flex-grow overflow-y-auto">
        <ul class="list-none p-0">
            {#each $directoryStructure as node}
                <TreeNode {node} />
            {/each}
        </ul>
        <button class="w-full mt-2 border-1 border-blue-800 bg-blue-900 hover:border-blue-200 hover:bg-blue-800" on:click={createFolder}>+</button>
    </div>
</div>
