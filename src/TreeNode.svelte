<script lang="ts">
    import FaFileMedical from "svelte-icons/fa/FaFileMedical.svelte";
    import FaTrash from "svelte-icons/fa/FaTrash.svelte";
    import FaFolderPlus from "svelte-icons/fa/FaFolderPlus.svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { selectedFilePath, selectedCursor } from "./stores";
    import { invoke } from "@tauri-apps/api";

    export let path: string = "/";
    export let node: FileSystemNode;

    const isExpanded = writable(false);

    function toggleExpand(event: MouseEvent) {
        event.stopPropagation();
        if (node.type_ === "Directory") {
            isExpanded.update((value) => !value);
        }
    }

    function onFileClick(event: MouseEvent, filePath: string) {
        event.stopPropagation();
        selectedCursor.set(filePath);
        if (node.type_ === "File") {
            console.log(`File clicked: ${filePath}`);
            selectedFilePath.set(filePath);
        } else {
            selectedFilePath.set(filePath + "/_index.md");
        }
    }

    async function createFile(event: MouseEvent) {
        event.stopPropagation();
        try {
            await invoke("new_content_for_hugo", {
                filePath: `${path}${node.name}` + "/new_file.md",
            });
        } catch (error) {
            console.error("failed to create content:", error);
        }
        console.log("Create item");
    }
    function createFolder(event: MouseEvent) {
        event.stopPropagation();
        console.log("Create item");
        // 디렉터리 생성 로직 구현
    }

    function deleteItem(event: MouseEvent) {
        event.stopPropagation();
        console.log("Delete item");
        // 항목 삭제 로직 구현
    }
</script>

<li>
    <div class="flex items-center">
        {#if node.type_ === "Directory"}
            <button
                on:click={(event) => {
                    toggleExpand(event);
                    // onFileClick(event, `${path}${node.name}`);
                }}
                class="cursor-pointer style-button"
            >
                {$isExpanded ? "▼" : "▶︎"}
            </button>
        {/if}
        <button
            class="cursor-pointer flex-grow text-left overflow-hidden overflow-ellipsis whitespace-nowrap {$selectedCursor ===
            path + node.name
                ? 'bg-yellow-200'
                : ''}"
            on:click={(event) => onFileClick(event, `${path}${node.name}`)}
        >
            {node.name}
        </button>
        {#if $selectedCursor === path + node.name}
            {#if node.type_ === "Directory"}
                <button
                    on:click={createFile}
                    class="cursor-pointer w-4 h-4 ml-1"
                >
                    <FaFileMedical />
                </button>
                <button
                    on:click={createFolder}
                    class="cursor-pointer w-4 h-4 ml-1"
                >
                    <FaFolderPlus />
                </button>
            {/if}
            <button on:click={deleteItem} class="cursor-pointer w-4 h-4 ml-1">
                <FaTrash />
            </button>
        {/if}
    </div>
    {#if node.type_ === "Directory" && $isExpanded}
        <ul class="pl-4">
            {#each node.children as child}
                <TreeNode path={`${path}${node.name}/`} node={child} />
            {/each}
        </ul>
    {/if}
</li>
