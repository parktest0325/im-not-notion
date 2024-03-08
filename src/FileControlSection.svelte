<script lang="ts">
    import { invoke } from "@tauri-apps/api";
    import FaSearch from "svelte-icons/fa/FaSearch.svelte";
    import IoMdRefresh from "svelte-icons/io/IoMdRefresh.svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";

    let searchTerm: string = "";
    let directoryStructure = writable<FileSystemNode[]>([]);

    const refreshList = async () => {
        const data: FileSystemNode = await invoke("get_file_list");
        directoryStructure.set(data.children);
        console.log(data);
    };

    const searchFiles = (term: string) => {
        // 검색 로직
    };
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
    </div>
</div>
