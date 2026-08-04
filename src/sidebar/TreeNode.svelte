<script lang="ts">
    import { FilePlus, FolderPlus, Trash2 } from "lucide-svelte";
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { relativeFilePath, selectedCursor, draggingInfo, isEditingFileName, renamingPath, addToast } from "../stores";
    import { dropTargetPath, onNodePointerDown, HOVER_EXPAND_MS } from "./treeDrag";
    import { onDestroy } from "svelte";
    import { type GlobalFunctions, GLOBAL_FUNCTIONS } from "../context";
    import { invoke } from "@tauri-apps/api/core";
    import { getContext } from "svelte";
    import { slide } from "svelte/transition";
    import { NodeType, type FileSystemNode } from "../types/setting";
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
        if (node.type_ === NodeType.Directory) {
            isExpanded.update((value) => !value);
        }
    }

    function onFileClick(event: MouseEvent) {
        event.stopPropagation();
        selectedCursor.set(filePath);
        if (node.type_ === NodeType.File) {
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
                    node.type_ === NodeType.Directory
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

    // renamingPath store가 이 노드의 경로와 일치하면 편집 모드 진입
    $: if ($renamingPath === filePath && !isEditing) {
        isEditing = true;
        isEditingFileName.set(true);
        editableName = node.name;
        renamingPath.set("");
    }

    $: isDragging = $draggingInfo?.path === filePath;

    // 드래그 중 닫힌 폴더 위에 잠시 머물면 자동으로 펼침
    let hoverExpandTimer: ReturnType<typeof setTimeout> | null = null;
    $: {
        if ($draggingInfo && $dropTargetPath === filePath
            && node.type_ === NodeType.Directory && !$isExpanded) {
            if (!hoverExpandTimer) {
                hoverExpandTimer = setTimeout(() => {
                    hoverExpandTimer = null;
                    isExpanded.set(true);
                }, HOVER_EXPAND_MS);
            }
        } else if (hoverExpandTimer) {
            clearTimeout(hoverExpandTimer);
            hoverExpandTimer = null;
        }
    }

    onDestroy(() => {
        if (hoverExpandTimer) clearTimeout(hoverExpandTimer);
    });
</script>

<li data-drop-dir={node.type_ === NodeType.Directory ? filePath : undefined}
    class:drag-over-target={$dropTargetPath === filePath}
    class:dragging={isDragging}
    on:pointerdown={(e) => onNodePointerDown(e, filePath)}
    >
    <div class="flex items-center">
        {#if node.type_ === NodeType.Directory}
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
            {#if node.type_ === NodeType.Directory}
                <button
                    on:click={(event) => createItem(event, "File")}
                    class="cursor-pointer w-4 h-4 ml-1"
                >
                    <FilePlus size="100%" />
                </button>
                <button
                    on:click={(event) => createItem(event, "Directory")}
                    class="cursor-pointer w-4 h-4 ml-1"
                >
                    <FolderPlus size="100%" />
                </button>
            {/if}
            <button
                on:click={confirmDeleteItem}
                class="cursor-pointer w-4 h-4 ml-1"
            >
                <Trash2 size="100%" />
            </button>
        {/if}
    </div>

    {#if showDeleteConfirmation}
        <div
            transition:slide={{ duration: 300 }}
            class="mt-2 p-3 rounded-md border-2"
            style="background-color: var(--confirm-box-bg); color: var(--confirm-box-text); border-color: var(--confirm-box-border);"
        >
            <p class="text-sm">Are you sure you want to delete this item?</p>
            <div class="flex justify-end space-x-2 mt-2">
                <button
                    class="px-4 py-1 rounded btn-danger focus:outline-none"
                    on:click={() => proceedDelete(true)}
                >
                    Yes
                </button>
                <button
                    class="px-4 py-1 rounded btn-cancel focus:outline-none"
                    on:click={() => proceedDelete(false)}
                >
                    No
                </button>
            </div>
        </div>
    {/if}
    {#if node.type_ === NodeType.Directory && $isExpanded}
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
    /* 드롭 대상: 흐려지는 대신 뚜렷한 테두리 + 배경 강조로 표시 */
    .drag-over-target {
        outline: 2px dashed var(--accent-color);
        outline-offset: -2px;
        border-radius: 0.25rem;
        background-color: var(--button-hover-bg-color);
    }
    .dragging {
        opacity: 0.5;
    }
    .text-hidden {
        color: var(--reverse-third-color);
        font-style: italic;
    }
    .bg-selected-file.text-hidden {
        color: var(--reverse-third-color-selected);
    }
    .btn-danger {
        background-color: var(--btn-danger-bg);
        color: var(--btn-danger-text);
    }
    .btn-danger:hover {
        background-color: var(--btn-danger-hover-bg);
    }
    .btn-cancel {
        background-color: var(--btn-cancel-bg);
        color: var(--btn-cancel-text);
    }
    .btn-cancel:hover {
        background-color: var(--btn-cancel-hover-bg);
    }
</style>