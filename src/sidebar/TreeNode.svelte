<script lang="ts">
    import FaFileMedical from "svelte-icons/fa/FaFileMedical.svelte";
    import FaTrash from "svelte-icons/fa/FaTrash.svelte";
    import FaFolderPlus from "svelte-icons/fa/FaFolderPlus.svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { selectedFilePath, selectedCursor } from "../stores";
    import { invoke } from "@tauri-apps/api";
    import { getContext, onDestroy, onMount } from "svelte";
    import { slide } from "svelte/transition";
    import DiJenkins from 'svelte-icons/di/DiJenkins.svelte'
    import DiComposer from 'svelte-icons/di/DiComposer.svelte'

    export let path: string = "/";
    export let node: FileSystemNode;
    let filenameInput: HTMLInputElement;
    interface GlobalFunctions {
        refreshList: () => Promise<void>;
    }

    // getContext를 사용하여 전역 함수를 가져옴
    const { refreshList } = getContext<GlobalFunctions>("globalFunctions");

    const isExpanded = writable(false);

    $: filePath = `${path}${node.name}`;

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
                // createdPath = filePath + "/new_folder";
                // await invoke("make_directory", {
                //     path: createdPath,
                // });
                createdPath = filePath + "/new_folder/_index.md";
                await invoke("new_content_for_hugo", {
                    filePath: createdPath,
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
        isEditing = false;
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
            // await invoke("move_to_trashcan", {
            await invoke("remove_file", {
                path: filePath,
            });
            selectedCursor.set("");
            selectedFilePath.set("");
            await refreshList();
        } catch (error) {
            console.error("failed to rmrf:", error);
        }
        console.log("Delete item");
    }

    let isEditing = false;
    let editableName = node.name;

    // 변경 사항 저장 또는 취소
    async function handleEdit(event: KeyboardEvent) {
        if (event.key === "Enter") {
            isEditing = false;
            event.preventDefault(); // 이벤트의 기본 동작 방지
            event.stopPropagation(); // 이벤트의 전파 방지
            try {
                const dstPath = path + editableName;
                await invoke("move_file_or_folder", {
                    src: filePath,
                    dst: dstPath,
                });
                node.name = editableName;
                selectedCursor.set(dstPath);
                selectedFilePath.set(
                    node.type_ === "Directory"
                        ? dstPath + "/_index.md"
                        : dstPath,
                );
                await refreshList();
            } catch (error) {
                console.error("Failed to rename file:", error);
            }
        } else if (event.key === "Escape") {
            isEditing = false;
            editableName = node.name; // 변경을 취소하고 원래 이름으로 복원
        }
    }

    // true일때만 filenameInput에 포커스
    $: if (isEditing) {
        filenameInput?.focus();
    }

    // 선택된 파일명 편집을 위해 F2 또는 Enter 키 이벤트 활성화
    function onKeyDown(event: KeyboardEvent) {
        if (
            $selectedCursor === filePath &&
            !isEditing &&
            (event.key === "F2" || event.key === "Enter")
        ) {
            isEditing = true;
            // 위의 노드가 삭제됐을때 리렌더링이 되면서
            // editableName이 기존 input 위치의 editableName으로 변해서 강제로 저
            editableName = node.name;
        }
    }

    onMount(() => {
        document.addEventListener("keydown", onKeyDown);
    });

    onDestroy(() => {
        document.removeEventListener("keydown", onKeyDown);
    });
</script>

<li>
    <div class="flex items-center">
        {#if node.type_ === "Directory"}
            <button
                on:click={(event) => {
                    toggleExpand(event);
                    // onFileClick(event, `${path}${node.name}`);
                }}
                class="cursor-pointer w-6 h-6 rounded"
            >
            {#if $isExpanded}
                <img src="/invader_open.svg" alt="open" />
            {:else}
                <img src="/invader_close.svg" alt="close"/>
            {/if}
            </button>
        {/if}

        {#if isEditing}
            <input
                bind:this={filenameInput}
                class="pl-2 pr-2"
                type="text"
                bind:value={editableName}
                on:keydown={handleEdit}
                on:blur={() => {
                    isEditing = false;
                }}
            />
        {:else}
            <button
                class="pl-2 pr-2 font-bold cursor-pointer flex-grow text-left overflow-hidden overflow-ellipsis whitespace-nowrap {$selectedCursor ===
                path + node.name
                    ? 'bg-yellow-200'
                    : ''}"
                on:click={onFileClick}
            >
                {node.name}
            </button>
        {/if}

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
