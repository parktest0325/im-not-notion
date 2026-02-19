<script lang="ts">
    import FaFileMedical from "svelte-icons/fa/FaFileMedical.svelte";
    import FaTrash from "svelte-icons/fa/FaTrash.svelte";
    import FaFolderPlus from "svelte-icons/fa/FaFolderPlus.svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { relativeFilePath, selectedCursor, draggingInfo, isEditingFileName, addToast, type GlobalFunctions, GLOBAL_FUNCTIONS } from "../stores";
    import { invoke } from "@tauri-apps/api/core";
    import { getContext, onDestroy, onMount } from "svelte";
    import { slide } from "svelte/transition";
    import type { FileSystemNode } from "../types/setting";
    import FolderClose from '../resource/InvaderClose.svelte';
    import FolderOpen from '../resource/InvaderOpen.svelte';

    export let path: string = "/";
    export let node: FileSystemNode;
    let filenameInput: HTMLInputElement;

    // getContext를 사용하여 전역 함수를 가져옴
    const { refreshList } = getContext<GlobalFunctions>(GLOBAL_FUNCTIONS);

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
            relativeFilePath.set(filePath);
        } else {
            relativeFilePath.set(filePath + "/_index.md");
        }
    }

    async function createItem(event: MouseEvent, createType: string) {
        event.stopPropagation();
        try {
            const basePath = createType === "Directory"
                ? filePath + "/new_folder/_index.md"
                : filePath + "/new_file.md";
            const createdPath: string = await invoke("new_content_for_hugo", {
                filePath: basePath,
            });
            isExpanded.set(true);
            selectedCursor.set(createdPath);
            relativeFilePath.set(createdPath);
            await refreshList();
            addToast("Item created.", "success");
        } catch (error) {
            console.error("failed to create item:", error);
            addToast("Failed to create item.");
        }
    }

    $: if ($selectedCursor) {
        showDeleteConfirmation = false;
        isEditing = false;
        isEditingFileName.set(false);
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
            await invoke("remove_file", {
                path: filePath,
            });
            selectedCursor.set("");
            relativeFilePath.set("");
            await refreshList();
            addToast("Item deleted.", "success");
        } catch (error) {
            console.error("failed to rmrf:", error);
            addToast("Failed to delete item.");
        }
    }

    let isEditing = false;
    let editableName = node.name;

    // 변경 사항 저장 또는 취소
    async function handleEdit(event: KeyboardEvent) {
        if (event.key === "Enter") {
            isEditing = false;
            isEditingFileName.set(false);
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
                relativeFilePath.set(
                    node.type_ === "Directory"
                        ? dstPath + "/_index.md"
                        : dstPath,
                );
                await refreshList();
                addToast("Item renamed.", "success");
            } catch (error) {
                console.error("Failed to rename file:", error);
                addToast("Failed to rename item.");
            }
        } else if (event.key === "Escape") {
            isEditing = false;
            isEditingFileName.set(false);
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
            isEditingFileName.set(true);
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

    let isDragOver = false;
    $: isDragging = $draggingInfo?.path === filePath;
    $: dragDisabled = $isEditingFileName;

    function onDragStart(event: DragEvent) {
        if (dragDisabled) return;
        event.stopPropagation();
        event.dataTransfer?.setData('application/x-imnotnotion-path', filePath);
        draggingInfo.set({ path: filePath });
    }

    function onDragEnd(event: DragEvent) {
        if (dragDisabled) return;
        event.stopPropagation();
        draggingInfo.set(null);
        isDragOver = false;
    }

    function onDragOver(event: DragEvent) {
        if (dragDisabled) return;
        event.stopPropagation();
        if (node.type_ === 'Directory') {
            event.preventDefault();
            event.dataTransfer!.dropEffect = 'move';
        }
    }

    function onDragEnter(event: DragEvent) {
        if (dragDisabled) return;
        event.stopPropagation();
        if (node.type_ === 'Directory') {
            event.preventDefault();
            isDragOver = true;
        }
    }

    function onDragLeave(event: DragEvent) {
        if (dragDisabled) return;
        event.stopPropagation();
        isDragOver = false;
    }

    async function onDrop(event: DragEvent) {
        if (dragDisabled || node.type_ !== 'Directory') return;
        event.stopPropagation();
        event.preventDefault();

        isDragOver = false;
        const info = $draggingInfo;
        if (!info || info.path === filePath) return;

        const src = info.path;
        const name = src.split('/').pop();
        const dst  = `${filePath}/${name}`;

        try {
            await invoke('move_file_or_folder', { src, dst });
            selectedCursor.set(dst);
            relativeFilePath.set(dst);
            await refreshList();
            addToast("Item moved.", "success");
        } catch (e) {
            console.error('Failed to move file:', e);
            addToast("Failed to move item.");
        }
    }
</script>

<li draggable={!dragDisabled}
    class:drag-over-target={isDragOver}
    class:dragging={isDragging}
    on:dragstart={onDragStart}
    on:dragend={onDragEnd}
    on:dragenter={onDragEnter}
    on:dragover={onDragOver}
    on:dragleave={onDragLeave}
    on:drop={onDrop}
    >
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
                <FolderOpen />
            {:else}
                <FolderClose />
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
                    isEditingFileName.set(false);
                }}
            />
        {:else}
            <button
                class="pl-2 pr-2 font-bold cursor-pointer flex-grow text-left overflow-hidden overflow-ellipsis whitespace-nowrap
                {$selectedCursor === filePath
                    ? 'bg-selected-file'
                    : ''} {node.is_hidden ? 'text-hidden' : ''}"
                on:click={onFileClick}
            >
                {node.name}
            </button>
        {/if}

        {#if $selectedCursor === filePath && !$isEditingFileName}
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


<style>
    .bg-selected-file {
        background-color: var(--button-selected-bg-color);
        color: var(--button-selected-text-color);
    }
    .drag-over-target {
        background-color: var(--button-selected-bg-color);
        opacity: 0.5;
    }
    .dragging {
        opacity: 0.5;
    }
    .text-hidden {
        color: var(--reverse-third-color);
    }
</style>