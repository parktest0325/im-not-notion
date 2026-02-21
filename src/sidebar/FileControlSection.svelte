<script context="module" lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { writable } from "svelte/store";
    import { isConnected, addToast } from "../stores";
    import type { FileSystemNode } from "../types/setting";

    let directoryStructure = writable<FileSystemNode[]>([]);
    export async function refreshList() {
        try {
            const data: FileSystemNode[] = await invoke("get_file_tree");
            directoryStructure.set(data);
            isConnected.set(true);
        } catch (error) {
            console.error("Failed to update file list:", error);
            directoryStructure.set([]);
            const connected: boolean = await invoke("check_connection");
            isConnected.set(connected);
            if (!connected) {
                addToast("SSH connection lost.");
            } else {
                addToast("Failed to load file list.");
            }
        }
    }
</script>

<script lang="ts">
    import FaSearch from "svelte-icons/fa/FaSearch.svelte";
    import IoMdRefresh from "svelte-icons/io/IoMdRefresh.svelte";
    import FaFileMedical from "svelte-icons/fa/FaFileMedical.svelte";
    import FaFolderPlus from "svelte-icons/fa/FaFolderPlus.svelte";
    import TreeNode from "./TreeNode.svelte";
    import { onMount } from "svelte";
    import { selectedCursor, relativeFilePath, draggingInfo, isEditingFileName } from "../stores";

    let searchTerm: string = "";
    let expandedSections: Record<string, boolean> = {};
    let dragOverSection: string | null = null;

    onMount(refreshList);

    // 섹션이 로드되면 기본적으로 모두 펼침
    $: {
        for (const node of $directoryStructure) {
            if (!(node.name in expandedSections)) {
                expandedSections[node.name] = true;
            }
        }
    }

    function toggleSection(name: string) {
        expandedSections[name] = !expandedSections[name];
        expandedSections = expandedSections;
    }

    async function createInSection(event: MouseEvent, sectionName: string, createType: string) {
        event.stopPropagation();
        try {
            const basePath = createType === "Directory"
                ? `/${sectionName}/new_folder/_index.md`
                : `/${sectionName}/new_file.md`;
            const createdPath: string = await invoke("new_content_for_hugo", {
                filePath: basePath,
            });
            selectedCursor.set(createdPath);
            relativeFilePath.set(createdPath);
            expandedSections[sectionName] = true;
            expandedSections = expandedSections;
            await refreshList();
            addToast("Item created.", "success");
        } catch (error) {
            console.error("failed to create item:", error);
            addToast("Failed to create item.");
        }
    }

    function onSectionDragOver(event: DragEvent, sectionName: string) {
        if ($isEditingFileName) return;
        event.preventDefault();
        event.stopPropagation();
        event.dataTransfer!.dropEffect = 'move';
        dragOverSection = sectionName;
    }

    function onSectionDragLeave(event: DragEvent) {
        event.stopPropagation();
        dragOverSection = null;
    }

    async function onSectionDrop(event: DragEvent, sectionName: string) {
        if ($isEditingFileName) return;
        event.preventDefault();
        event.stopPropagation();
        dragOverSection = null;

        const info = $draggingInfo;
        if (!info) return;

        const src = info.path;
        const name = src.split('/').pop();
        const dst = `/${sectionName}/${name}`;

        // 같은 위치면 무시
        if (src === dst) return;

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

    const searchFiles = (_term: string) => {
        // TODO: 검색 로직 구현
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

    <!-- 섹션별 파일 리스트 -->
    <div class="flex-grow overflow-y-auto">
        {#each $directoryStructure as section, i}
            {#if i > 0}
                <hr class="section-divider" />
            {/if}
            <div class="section">
                <button class="section-header"
                    class:drag-over-section={dragOverSection === section.name}
                    on:click={() => toggleSection(section.name)}
                    on:dragover={(e) => onSectionDragOver(e, section.name)}
                    on:dragleave={onSectionDragLeave}
                    on:drop={(e) => onSectionDrop(e, section.name)}
                >
                    <span class="section-arrow">{expandedSections[section.name] ? '\u25BC' : '\u25B6'}</span>
                    <span class="section-name">{section.name}</span>
                    <span class="section-actions">
                        <button
                            class="section-action-btn"
                            on:click|stopPropagation={(e) => createInSection(e, section.name, "File")}
                            title="New file"
                        >
                            <div class="w-3 h-3"><FaFileMedical /></div>
                        </button>
                        <button
                            class="section-action-btn"
                            on:click|stopPropagation={(e) => createInSection(e, section.name, "Directory")}
                            title="New folder"
                        >
                            <div class="w-3 h-3"><FaFolderPlus /></div>
                        </button>
                    </span>
                </button>
                {#if expandedSections[section.name]}
                    <ul class="list-none p-0">
                        {#each section.children as node}
                            <TreeNode path={`/${section.name}/`} {node} />
                        {/each}
                    </ul>
                {/if}
            </div>
        {/each}
    </div>
</div>

<style>
    .section-divider {
        border: none;
        border-top: 1px solid var(--border-color);
        margin: 0.25rem 0;
    }

    .section-header {
        display: flex;
        align-items: center;
        width: 100%;
        padding: 0.375rem 0.25rem;
        border: none;
        background: none;
        cursor: pointer;
        font-size: 0.8rem;
        font-weight: 600;
        opacity: 0.7;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        box-shadow: none;
    }

    .section-header:hover {
        opacity: 1;
    }

    .section-arrow {
        font-size: 0.6rem;
        width: 1rem;
        flex-shrink: 0;
    }

    .section-name {
        flex-grow: 1;
        text-align: left;
    }

    .section-actions {
        display: flex;
        gap: 0.25rem;
        opacity: 0;
        transition: opacity 0.15s;
    }

    .section-header:hover .section-actions {
        opacity: 1;
    }

    .section-action-btn {
        padding: 0.125rem;
        border: none;
        background: none;
        cursor: pointer;
        opacity: 0.6;
        border-radius: 0.25rem;
        box-shadow: none;
    }

    .section-action-btn:hover {
        opacity: 1;
        background-color: var(--button-hover-bg-color);
    }

    .drag-over-section {
        background-color: var(--button-selected-bg-color);
        opacity: 0.5;
    }
</style>
