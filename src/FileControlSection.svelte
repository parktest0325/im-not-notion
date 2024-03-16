<script lang="ts">
    import { invoke } from "@tauri-apps/api";
    import FaSearch from "svelte-icons/fa/FaSearch.svelte";
    import IoMdRefresh from "svelte-icons/io/IoMdRefresh.svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { setContext } from "svelte";
    import { selectedCursor, selectedFilePath } from "./stores";

    let searchTerm: string = "";
    let directoryStructure = writable<FileSystemNode[]>([]);

    setContext("globalFunctions", {
        refreshList,
    });
    export async function refreshList() {
        const data: FileSystemNode = await invoke("get_file_list");
        directoryStructure.set(data.children);
        console.log(data);
    }

    const searchFiles = (term: string) => {
        // 검색 로직
    };

    async function createFolder(event: MouseEvent) {
        event.stopPropagation();
        try {
            // const createdPath = "/new_folder";
            // await invoke("make_directory", {
            //     path: createdPath,
            // });
            const createdPath = "/new_folder/_index.md";
            await invoke("new_content_for_hugo", {
                filePath: createdPath,
            });
            selectedCursor.set(createdPath);
            selectedFilePath.set(createdPath);
            await refreshList();
        } catch (error) {
            console.error("failed to make directory:", error);
        }
        console.log("Create folder");
        // 디렉터리 생성 로직 구현
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
        <button class="w-full" on:click={createFolder}>+</button>
    </div>
</div>
