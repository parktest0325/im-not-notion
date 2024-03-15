<script lang="ts">
    import FaFileMedical from "svelte-icons/fa/FaFileMedical.svelte";
    import FaTrash from "svelte-icons/fa/FaTrash.svelte";
    import FaFolderPlus from "svelte-icons/fa/FaFolderPlus.svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { selectedFilePath, selectedCursor } from "./stores";
    import { invoke } from "@tauri-apps/api";
    import { getContext } from "svelte";
    import { slide } from "svelte/transition";
    export let path: string = "/";
    export let node: FileSystemNode;

    interface GlobalFunctions {
        refreshList: () => Promise<void>;
    }

    // getContext를 사용하여 전역 함수를 가져옴
    const { refreshList } = getContext<GlobalFunctions>("globalFunctions");

    const isExpanded = writable(false);

    const filePath = `${path}${node.name}`;

    function toggleExpand(event: MouseEvent) {
        event.stopPropagation();
        if (node.type_ === "Directory") {
            isExpanded.update((value) => !value);
        }
    }

    function onFileClick(event: MouseEvent) {
        event.stopPropagation();
        selectedCursor.set(filePath);
        if (node.type_ === "File") {
            console.log(`File clicked: ${filePath}`);
            selectedFilePath.set(filePath);
        } else {
            selectedFilePath.set(filePath + "/_index.md");
        }
    }

    async function createItem(event: MouseEvent, createType: string) {
        event.stopPropagation();
        try {
            let createdPath: string;
            if (createType === "Directory") {
                createdPath = filePath + "/new_folder";
                await invoke("make_directory", {
                    path: createdPath,
                });
            } else {
                createdPath = filePath + "/new_file.md";
                await invoke("new_content_for_hugo", {
                    filePath: createdPath,
                });
            }
            isExpanded.set(true);
            selectedCursor.set(createdPath);
            selectedFilePath.set(createdPath);
            await refreshList();
        } catch (error) {
            console.error("failed to create item:", error);
        }
        console.log("Create item");
    }

    $: if ($selectedCursor) {
        showDeleteConfirmation = false;
    }

    let showDeleteConfirmation = false;

    function confirmDeleteItem(event: MouseEvent) {
        event.stopPropagation();
        showDeleteConfirmation = true;
    }

    async function proceedDelete(confirmation: boolean) {
        if (confirmation) {
            await deleteItem();
        }
        showDeleteConfirmation = false;
    }

    async function deleteItem() {
        try {
            await invoke("move_to_trashcan", {
                path: filePath,
            });
            selectedCursor.set("");
            selectedFilePath.set("");
            await refreshList();
        } catch (error) {
            console.error("failed to move trashcan:", error);
        }
        console.log("Delete item");
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
            on:click={onFileClick}
        >
            {node.name}
        </button>

        {#if $selectedCursor === path + node.name}
            {#if node.type_ === "Directory"}
                <button
                    on:click={(event) => createItem(event, "File")}
                    class="cursor-pointer w-4 h-4 ml-1"
                >
                    <FaFileMedical />
                </button>
                <button
                    on:click={(event) => createItem(event, "Directory")}
                    class="cursor-pointer w-4 h-4 ml-1"
                >
                    <FaFolderPlus />
                </button>
            {/if}
            <button
                on:click={confirmDeleteItem}
                class="cursor-pointer w-4 h-4 ml-1"
            >
                <FaTrash />
            </button>
        {/if}
    </div>

    {#if showDeleteConfirmation}
        <div
            transition:slide={{ duration: 300 }}
            class="mt-2 bg-gray-800 text-white p-3 rounded-md border-2 border-red-500"
        >
            <p class="text-sm">Are you sure you want to delete this item?</p>
            <div class="flex justify-end space-x-2 mt-2">
                <button
                    class="px-4 py-1 bg-red-500 text-white rounded hover:bg-red-700 focus:outline-none focus:ring focus:ring-red-300"
                    on:click={() => proceedDelete(true)}
                >
                    Yes
                </button>
                <button
                    class="px-4 py-1 bg-gray-300 text-gray-800 rounded hover:bg-gray-400 focus:outline-none focus:ring focus:ring-gray-300"
                    on:click={() => proceedDelete(false)}
                >
                    No
                </button>
            </div>
        </div>
    {/if}
    {#if node.type_ === "Directory" && $isExpanded}
        <ul class="pl-4">
            {#each node.children as child}
                <TreeNode path={`${filePath}/`} node={child} />
            {/each}
        </ul>
    {/if}
</li>
