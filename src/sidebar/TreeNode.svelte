<script lang="ts">
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { relativeFilePath, selectedCursor, draggingInfo, isEditingFileName, renamingPath, addToast, treeExpandSignal, treeContextMenu, renameOpenTabs } from "../stores";
    import { dropTargetPath, onNodePointerDown, HOVER_EXPAND_MS } from "./treeDrag";
    import { onDestroy } from "svelte";
    import { type GlobalFunctions, GLOBAL_FUNCTIONS } from "../context";
    import { invoke } from "@tauri-apps/api/core";
    import { getContext } from "svelte";
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

    $: if ($selectedCursor) {
        isEditing = false;
        isEditingFileName.set(false);
    }

    function onContextMenu(event: MouseEvent) {
        event.preventDefault();
        event.stopPropagation();
        treeContextMenu.set({
            x: event.clientX,
            y: event.clientY,
            path: filePath,
            isDir: node.type_ === NodeType.Directory,
        });
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
                // 열려있는 탭 경로 갱신 (폴더면 하위 탭도 함께) — 스토어 갱신 전에
                renameOpenTabs(filePath, dstPath);
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
    // 컨텍스트 메뉴가 이 항목을 대상으로 열려있는지
    $: isMenuTarget = $treeContextMenu?.path === filePath;

    // 섹션 전체 펼치기/접기 신호 적용 (펼치기 중 새로 마운트되는 하위 노드도
    // 같은 flush 안에서 신호를 읽어 연쇄적으로 펼쳐진다)
    // exact=true면 해당 경로의 폴더 하나만 펼친다
    $: if ($treeExpandSignal && node.type_ === NodeType.Directory
        && ($treeExpandSignal.exact
            ? filePath === $treeExpandSignal.prefix
            : (filePath + "/").startsWith($treeExpandSignal.prefix + "/"))) {
        isExpanded.set($treeExpandSignal.expand);
    }

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
    on:contextmenu={onContextMenu}
    >
    <div class="flex items-center" class:menu-target={isMenuTarget}>
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

    </div>

    {#if node.type_ === NodeType.Directory && $isExpanded}
        <ul class="pl-4">
            {#each node.children as child}
                <TreeNode path={`${filePath}/`} node={child} />
            {/each}
        </ul>
    {/if}
</li>


<style>
    /* 트리 항목은 플랫하게 — 전역 버튼 스타일(둥근 테두리/그림자)이
       새어 들어와 어중간한 둥근 네모로 보이는 것 방지 */
    li :global(button) {
        border: none;
        background: none;
        box-shadow: none;
        border-radius: 3px;
        padding-top: 1px;
        padding-bottom: 1px;
    }
    li :global(button:hover) {
        border: none;
        background-color: var(--button-hover-bg-color);
    }

    /* 우클릭 메뉴의 대상 항목 표시 */
    .menu-target {
        outline: 1px solid var(--accent-color);
        outline-offset: -1px;
        border-radius: 3px;
        background-color: var(--button-hover-bg-color);
    }

    .bg-selected-file {
        background-color: var(--button-selected-bg-color);
        color: var(--button-selected-text-color);
    }
    .bg-selected-file:hover {
        background-color: var(--button-selected-bg-color);
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
</style>