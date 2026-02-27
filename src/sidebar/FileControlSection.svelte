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
    import { selectedCursor, relativeFilePath, draggingInfo, isEditingFileName, gotoLine } from "../stores";

    interface SearchMatch {
        file_path: string;
        line_num: number;
        line_text: string;
    }

    interface GroupedResult {
        file_path: string;
        matches: { line_num: number; line_text: string }[];
    }

    let searchTerm: string = "";
    let activeSection: string | null = null;
    let initialized = false;
    let dragOverSection: string | null = null;

    let searchResults: GroupedResult[] = [];
    let isSearching = false;
    let hasSearched = false;
    let activeResultKey = "";

    onMount(refreshList);

    // 최초 로드 시에만 첫 번째 섹션 활성화
    $: {
        if ($directoryStructure.length > 0 && !initialized) {
            activeSection = $directoryStructure[0].name;
            initialized = true;
        }
    }

    function toggleSection(name: string) {
        if (activeSection === name) {
            activeSection = null;
        } else {
            activeSection = name;
        }
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
            activeSection = sectionName;
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

    function groupByFile(matches: SearchMatch[]): GroupedResult[] {
        const map = new Map<string, { line_num: number; line_text: string }[]>();
        for (const m of matches) {
            let arr = map.get(m.file_path);
            if (!arr) {
                arr = [];
                map.set(m.file_path, arr);
            }
            arr.push({ line_num: m.line_num, line_text: m.line_text });
        }
        return Array.from(map.entries()).map(([file_path, matches]) => ({ file_path, matches }));
    }

    async function doSearch() {
        const query = searchTerm.trim();
        if (!query) {
            searchResults = [];
            hasSearched = false;
            return;
        }
        isSearching = true;
        try {
            const raw: SearchMatch[] = await invoke("search_content_cmd", { query });
            searchResults = groupByFile(raw);
            hasSearched = true;
        } catch (e) {
            console.error("search_content_cmd error:", e);
            addToast("Search failed.");
            searchResults = [];
        } finally {
            isSearching = false;
        }
    }

    function clearSearch() {
        searchTerm = "";
        searchResults = [];
        hasSearched = false;
    }

    function onSearchKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            doSearch();
        } else if (e.key === "Escape") {
            clearSearch();
        }
    }

    function openSearchResult(filePath: string, lineNum: number) {
        activeResultKey = `${filePath}:${lineNum}`;
        selectedCursor.set(filePath);
        relativeFilePath.set(filePath);
        gotoLine.set(lineNum);
    }

    /** Extract display name from file path: "/blog/my-post/_index.md" → "my-post" */
    function displayName(filePath: string): string {
        const parts = filePath.split('/').filter(Boolean);
        // If ends with _index.md, use parent folder name
        if (parts.length >= 2 && parts[parts.length - 1] === '_index.md') {
            return parts[parts.length - 2];
        }
        // Otherwise use filename without extension
        const last = parts[parts.length - 1] || filePath;
        return last.replace(/\.md$/, '');
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
            on:keydown={onSearchKeydown}
            style="min-width: 0; width: auto; flex-grow: 1;"
        />
        <button on:click={doSearch} title="Search">
            <div class="w-5 h-5">
                <FaSearch />
            </div>
        </button>
        <button on:click={refreshList} title="Refresh">
            <div class="w-5 h-5">
                <IoMdRefresh />
            </div>
        </button>
    </div>

    {#if hasSearched}
        <!-- 검색 결과 헤더 (고정) -->
        {#if isSearching}
            <div class="search-status">Searching...</div>
        {:else if searchResults.length === 0}
            <div class="search-status">No results found.
                <button class="search-clear" on:click={clearSearch} title="Clear search">&times;</button>
            </div>
        {:else}
            <div class="search-header">
                <span>{searchResults.reduce((n, g) => n + g.matches.length, 0)} results in {searchResults.length} files</span>
                <button class="search-clear" on:click={clearSearch} title="Clear search">&times;</button>
            </div>
        {/if}
        <!-- 검색 결과 리스트 (스크롤) -->
        <div class="search-results">
            {#each searchResults as group}
                <div class="result-group">
                    <div class="result-file" title={group.file_path}>
                        {displayName(group.file_path)}
                        <span class="result-file-path">{group.file_path}</span>
                    </div>
                    {#each group.matches as m}
                        <button
                            class="result-line"
                            class:active={activeResultKey === `${group.file_path}:${m.line_num}`}
                            on:click={() => openSearchResult(group.file_path, m.line_num)}
                        >
                            <span class="result-linenum">L{m.line_num}</span>
                            <span class="result-text">{m.line_text}</span>
                        </button>
                    {/each}
                </div>
            {/each}
        </div>
    {:else}
        <!-- 섹션 아코디언: 헤더는 항상 보이고, 열린 섹션만 나머지 공간 차지 -->
        <div class="section-accordion">
            {#each $directoryStructure as section}
                <button class="section-header"
                    class:active={activeSection === section.name}
                    class:drag-over-section={dragOverSection === section.name}
                    on:click={() => toggleSection(section.name)}
                    on:dragover={(e) => onSectionDragOver(e, section.name)}
                    on:dragleave={onSectionDragLeave}
                    on:drop={(e) => onSectionDrop(e, section.name)}
                >
                    <span class="section-arrow">{activeSection === section.name ? '\u25BC' : '\u25B6'}</span>
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
                {#if activeSection === section.name}
                    <div class="section-content">
                        <ul class="list-none p-0">
                            {#each section.children as node}
                                <TreeNode path={`/${section.name}/`} {node} />
                            {/each}
                        </ul>
                    </div>
                {/if}
            {/each}
        </div>
    {/if}
</div>

<style>
    .section-accordion {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-height: 0;
        overflow: hidden;
    }

    .section-header {
        display: flex;
        align-items: center;
        width: 100%;
        padding: 0.375rem 0.25rem;
        border: none;
        border-top: 1px solid var(--border-color);
        background: none;
        cursor: pointer;
        font-size: 0.8rem;
        font-weight: 600;
        opacity: 0.7;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        box-shadow: none;
        flex-shrink: 0;
    }

    .section-header:first-child {
        border-top: none;
    }

    .section-header:hover {
        opacity: 1;
    }

    .section-header.active {
        opacity: 1;
        background-color: var(--button-hover-bg-color);
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

    .section-content {
        flex: 1;
        overflow-y: auto;
        min-height: 0;
    }

    .drag-over-section {
        background-color: var(--button-selected-bg-color);
        opacity: 0.5;
    }

    /* ── Search Results ── */

    .search-results {
        flex: 1;
        overflow-y: auto;
        min-height: 0;
    }

    .search-status {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.25rem;
        font-size: 0.75rem;
        opacity: 0.6;
        flex-shrink: 0;
    }

    .search-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.35rem 0.25rem;
        font-size: 0.75rem;
        font-weight: 600;
        border-bottom: 1px solid var(--border-color);
        flex-shrink: 0;
    }

    .search-clear {
        border: 1px solid var(--border-color);
        background: none;
        cursor: pointer;
        font-size: 1rem;
        color: inherit;
        padding: 0 0.35rem;
        border-radius: 0.25rem;
        box-shadow: none;
        line-height: 1.2;
    }
    .search-clear:hover {
        background-color: rgba(128, 128, 128, 0.2);
        border-color: var(--hover-border-color);
    }

    .result-group {
        margin-bottom: 0.25rem;
    }

    .result-file {
        padding: 0.25rem;
        font-size: 0.8rem;
        font-weight: 600;
        opacity: 0.85;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .result-file-path {
        font-weight: 400;
        opacity: 0.5;
        font-size: 0.7rem;
        margin-left: 0.25rem;
    }

    .result-line {
        display: flex;
        align-items: baseline;
        gap: 0.375rem;
        width: 100%;
        padding: 0.2rem 0.25rem 0.2rem 0.75rem;
        border: none;
        background: none;
        color: inherit;
        cursor: pointer;
        text-align: left;
        font-size: 0.78rem;
        border-radius: 0.2rem;
        box-shadow: none;
        transition: background-color 0.1s;
    }
    .result-line:hover {
        background-color: rgba(128, 128, 128, 0.15);
    }
    .result-line.active {
        background-color: rgba(13, 124, 135, 0.3);
    }
    .result-line:hover .result-linenum,
    .result-line.active .result-linenum {
        opacity: 0.8;
    }

    .result-linenum {
        flex-shrink: 0;
        opacity: 0.45;
        font-size: 0.7rem;
        min-width: 2rem;
    }

    .result-text {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
</style>
