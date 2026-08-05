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
    import { Search, RefreshCw, FilePlus, FolderPlus, ChevronsUpDown, ChevronsDownUp } from "lucide-svelte";
    import { treeExpandSignal } from "../stores";
    import { NodeType } from "../types/setting";
    import TreeNode from "./TreeNode.svelte";
    import { onMount, afterUpdate } from "svelte";
    import { selectedCursor, relativeFilePath, gotoLine } from "../stores";
    import { dropTargetPath, registerMoveHandler, HOVER_EXPAND_MS } from "./treeDrag";
    import { onDestroy } from "svelte";

    interface SearchMatch {
        file_path: string;
        line_num: number;
        line_text: string;
        is_hidden: boolean;
    }

    interface GroupedResult {
        file_path: string;
        is_hidden: boolean;
        matches: { line_num: number; line_text: string }[];
    }

    let searchTerm: string = "";
    let activeSection: string | null = null;
    let initialized = false;

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

    // 섹션 내 모든 폴더 펼치기/접기
    let expandSeq = 0;
    function setSectionExpanded(event: MouseEvent, sectionName: string, expand: boolean) {
        event.stopPropagation();
        if (expand) activeSection = sectionName; // 닫힌 섹션이면 먼저 열기
        treeExpandSignal.set({ prefix: `/${sectionName}`, expand, seq: ++expandSeq });
        // 연쇄 펼침은 같은 flush 안에서 끝나므로, 이후 마운트되는 노드에
        // 신호가 잔존 적용되지 않도록 바로 비운다
        setTimeout(() => treeExpandSignal.set(null), 0);
    }

    function toggleSection(name: string) {
        if (activeSection === name) {
            activeSection = null;
        } else {
            activeSection = name;
        }
    }

    // 드래그 중 닫힌 섹션 헤더 위에 잠시 머물면 자동으로 열기
    let sectionHoverTimer: ReturnType<typeof setTimeout> | null = null;
    let sectionHoverName: string | null = null;
    $: {
        const target = $dropTargetPath;
        const hovered = target
            ? $directoryStructure.find(s => `/${s.name}` === target)?.name ?? null
            : null;
        if (hovered && hovered !== activeSection) {
            if (sectionHoverName !== hovered) {
                if (sectionHoverTimer) clearTimeout(sectionHoverTimer);
                sectionHoverName = hovered;
                sectionHoverTimer = setTimeout(() => {
                    activeSection = sectionHoverName;
                    sectionHoverTimer = null;
                    sectionHoverName = null;
                }, HOVER_EXPAND_MS);
            }
        } else {
            if (sectionHoverTimer) {
                clearTimeout(sectionHoverTimer);
                sectionHoverTimer = null;
            }
            sectionHoverName = null;
        }
    }

    onDestroy(() => {
        if (sectionHoverTimer) clearTimeout(sectionHoverTimer);
    });

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

    registerMoveHandler(async (src: string, dstDir: string) => {
        const name = src.split('/').pop();
        const dst = `${dstDir}/${name}`;

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
    });

    function groupByFile(matches: SearchMatch[]): GroupedResult[] {
        const map = new Map<string, { is_hidden: boolean; items: { line_num: number; line_text: string }[] }>();
        for (const m of matches) {
            let entry = map.get(m.file_path);
            if (!entry) {
                entry = { is_hidden: m.is_hidden, items: [] };
                map.set(m.file_path, entry);
            }
            entry.items.push({ line_num: m.line_num, line_text: m.line_text });
        }
        return Array.from(map.entries()).map(([file_path, { is_hidden, items }]) => ({ file_path, is_hidden, matches: items }));
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
            const raw: SearchMatch[] = await invoke("search_content_cmd", { query, tags: searchTags, matchAll: tagMatchAll });
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

    // ── 태그 필터 (검색 결과 화면이 아니라 섹션 트리 자체를 필터링) ──
    let tagInput = "";
    let searchTags: string[] = [];
    let tagMatchAll = false; // 기본 OR
    let showTagPopover = false;
    let chipsEl: HTMLDivElement | null = null;
    let hiddenTagCount = 0;
    let tagFilterPaths: Set<string> | null = null;

    async function refreshTagFilter() {
        if (searchTags.length === 0) {
            tagFilterPaths = null;
            return;
        }
        try {
            const raw: SearchMatch[] = await invoke("search_content_cmd", {
                query: "",
                tags: searchTags,
                matchAll: tagMatchAll,
            });
            tagFilterPaths = new Set(raw.map((m) => m.file_path));
        } catch (e) {
            console.error("tag filter error:", e);
            addToast("Tag filter failed.");
        }
    }

    /** 태그 매칭 파일만 남기고 트리 필터링 (매칭 파일을 품은 조상 폴더는 유지) */
    function filterTree(nodes: FileSystemNode[], parentPath: string, keep: Set<string>): FileSystemNode[] {
        const out: FileSystemNode[] = [];
        for (const node of nodes) {
            const path = `${parentPath}/${node.name}`;
            if (node.type_ === NodeType.Directory) {
                const children = filterTree(node.children, path, keep);
                if (children.length > 0 || keep.has(`${path}/_index.md`)) {
                    out.push({ ...node, children });
                }
            } else if (keep.has(path)) {
                out.push(node);
            }
        }
        return out;
    }

    $: displayStructure = tagFilterPaths
        ? $directoryStructure.map((s) => ({
              ...s,
              children: filterTree(s.children, `/${s.name}`, tagFilterPaths!),
          }))
        : $directoryStructure;

    function onTagsChanged() {
        refreshTagFilter();
        // 텍스트 검색 결과가 떠 있으면 태그 제한을 반영해 갱신
        if (searchTerm.trim() && hasSearched) doSearch();
    }

    function addTag() {
        const t = tagInput.trim();
        tagInput = "";
        if (!t || searchTags.includes(t)) return;
        searchTags = [...searchTags, t];
        onTagsChanged();
    }

    function removeTag(tag: string) {
        searchTags = searchTags.filter((x) => x !== tag);
        if (searchTags.length === 0) showTagPopover = false;
        onTagsChanged();
    }

    function clearTags() {
        searchTags = [];
        showTagPopover = false;
        onTagsChanged();
    }

    /** 태그 문자열 해시 → hue (같은 태그는 항상 같은 색) */
    function tagHue(tag: string): number {
        let h = 0;
        for (let i = 0; i < tag.length; i++) {
            h = (h * 31 + tag.charCodeAt(i)) >>> 0;
        }
        return h % 360;
    }

    // 한 줄에 다 안 들어가는 칩 개수 측정 → [+n] 배지 표시용
    afterUpdate(() => {
        if (!chipsEl) {
            if (hiddenTagCount !== 0) hiddenTagCount = 0;
            return;
        }
        const limit = chipsEl.clientWidth; // padding-right가 배지 공간 확보
        let fit = 0;
        for (const el of Array.from(chipsEl.querySelectorAll<HTMLElement>(".tag-chip"))) {
            if (el.offsetLeft + el.offsetWidth <= limit) fit++;
        }
        const hidden = searchTags.length - fit;
        if (hidden !== hiddenTagCount) hiddenTagCount = hidden;
    });

    function onWindowClick(e: MouseEvent) {
        const t = e.target as HTMLElement;
        if (!t?.closest?.(".tag-popover, .tag-more")) showTagPopover = false;
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

<svelte:window on:click={onWindowClick} />

<div class="flex flex-col h-full" style="font-family: var(--font-mono);">
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
                <Search size="100%" />
            </div>
        </button>
        <button on:click={refreshList} title="Refresh">
            <div class="w-5 h-5">
                <RefreshCw size="100%" />
            </div>
        </button>
    </div>

    <!-- 태그 필터: Enter로 추가, 칩 클릭으로 제거, [+n] 클릭 시 전체 목록 말풍선 -->
    <div class="flex space-x-2 h-6 mb-2" style="flex-wrap: nowrap;">
        <input
            type="text"
            placeholder="Tag filter..."
            class="flex-grow p-2 border rounded"
            bind:value={tagInput}
            on:keydown={(e) => { if (e.key === "Enter") addTag(); }}
            style="min-width: 0; width: auto; flex-grow: 1;"
        />
        <button
            class="tag-mode-btn"
            title={tagMatchAll ? "모든 태그 포함(AND) — 클릭 시 OR" : "하나라도 포함(OR) — 클릭 시 AND"}
            on:click={() => { tagMatchAll = !tagMatchAll; if (searchTags.length > 0) onTagsChanged(); }}
        >
            {tagMatchAll ? "AND" : "OR"}
        </button>
        <button
            class="tag-mode-btn tag-clear-btn"
            title="Remove all tags"
            disabled={searchTags.length === 0}
            on:click={clearTags}
        >×</button>
    </div>
    {#if searchTags.length > 0}
        <div class="tag-chips-wrap">
            <div class="tag-chips" bind:this={chipsEl}>
                {#each searchTags as tag (tag)}
                    <button class="tag-chip" style="--tag-h: {tagHue(tag)}" title="Remove tag" on:click={() => removeTag(tag)}>{tag} ×</button>
                {/each}
            </div>
            {#if hiddenTagCount > 0}
                <button class="tag-more" title="Show all tags" on:click={() => (showTagPopover = !showTagPopover)}>+{hiddenTagCount}</button>
            {/if}
            {#if showTagPopover}
                <!-- 줄에서 가려진(+n) 태그들만 표시 -->
                <div class="tag-popover" role="dialog">
                    {#each searchTags.slice(searchTags.length - hiddenTagCount) as tag (tag)}
                        <button class="tag-chip" style="--tag-h: {tagHue(tag)}" title="Remove tag" on:click={() => removeTag(tag)}>{tag} ×</button>
                    {/each}
                </div>
            {/if}
        </div>
    {/if}

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
                    <div class="result-file {group.is_hidden ? 'text-hidden' : ''}" title={group.file_path}>
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
            {#each displayStructure as section}
                <button class="section-header"
                    data-drop-dir={`/${section.name}`}
                    class:active={activeSection === section.name}
                    class:drag-over-section={$dropTargetPath === `/${section.name}`}
                    on:click={() => toggleSection(section.name)}
                >
                    <span class="section-arrow">{activeSection === section.name ? '\u25BC' : '\u25B6'}</span>
                    <span class="section-name">{section.name}</span>
                    <span class="section-actions">
                        <button
                            class="section-action-btn"
                            on:click|stopPropagation={(e) => createInSection(e, section.name, "File")}
                            title="New file"
                        >
                            <div class="w-3 h-3"><FilePlus size="100%" /></div>
                        </button>
                        <button
                            class="section-action-btn"
                            on:click|stopPropagation={(e) => createInSection(e, section.name, "Directory")}
                            title="New folder"
                        >
                            <div class="w-3 h-3"><FolderPlus size="100%" /></div>
                        </button>
                        <button
                            class="section-action-btn"
                            on:click|stopPropagation={(e) => setSectionExpanded(e, section.name, true)}
                            title="Expand all"
                        >
                            <div class="w-3 h-3"><ChevronsUpDown size="100%" /></div>
                        </button>
                        <button
                            class="section-action-btn"
                            on:click|stopPropagation={(e) => setSectionExpanded(e, section.name, false)}
                            title="Collapse all"
                        >
                            <div class="w-3 h-3"><ChevronsDownUp size="100%" /></div>
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

    /* ── 태그 필터 ── */
    .tag-mode-btn {
        flex-shrink: 0;
        height: 100%;
        font-size: 10px;
        font-weight: 700;
        padding: 0 0.5rem;
        border-radius: 0.3rem;
        box-shadow: none;
    }
    .tag-mode-btn:disabled {
        opacity: 0.3;
        cursor: default;
    }
    /* 전체 제거: 파괴적 동작이므로 붉은 톤 (채도 낮게, 호버 시 진하게) */
    .tag-clear-btn:not(:disabled) {
        color: var(--error-color);
        opacity: 0.65;
    }
    .tag-clear-btn:not(:disabled):hover {
        opacity: 1;
        border-color: var(--error-color);
    }
    .tag-chips-wrap {
        position: relative;
        margin-bottom: 0.5rem;
    }
    .tag-chips {
        display: flex;
        gap: 4px;
        flex-wrap: nowrap;
        overflow: hidden;
        padding-right: 36px; /* [+n] 배지 자리 */
        min-height: 20px;
    }
    /* 태그별 색: 문자열 해시로 만든 --tag-h(hue) 기반 — 같은 태그는 항상 같은 색 */
    .tag-chip {
        font-size: 11px;
        line-height: 1.5;
        padding: 0 8px;
        border-radius: 999px;
        background-color: hsla(var(--tag-h), 65%, 50%, 0.16);
        color: hsl(var(--tag-h), 60%, 32%);
        border: 1px solid hsla(var(--tag-h), 60%, 45%, 0.35);
        box-shadow: none;
        white-space: nowrap;
        flex-shrink: 0;
    }
    .tag-chip:hover {
        border-color: hsl(var(--tag-h), 60%, 45%);
        background-color: hsla(var(--tag-h), 65%, 50%, 0.24);
    }
    :global(.dark) .tag-chip {
        color: hsl(var(--tag-h), 65%, 74%);
        border-color: hsla(var(--tag-h), 60%, 60%, 0.35);
    }
    :global(.dark) .tag-chip:hover {
        border-color: hsl(var(--tag-h), 60%, 60%);
    }
    .tag-more {
        position: absolute;
        right: 0;
        top: 0;
        font-size: 11px;
        line-height: 1.5;
        padding: 0 7px;
        border-radius: 999px;
        background-color: var(--tertiary-color);
        border: 1px solid var(--border-color);
        box-shadow: none;
    }
    /* [+n] 클릭 시 전체 태그 말풍선 */
    .tag-popover {
        position: absolute;
        top: calc(100% + 4px);
        right: 0;
        z-index: 40;
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
        max-width: 100%;
        padding: 0.5rem;
        background-color: var(--popup-bg-color);
        border: 1px solid var(--border-color);
        border-radius: 0.5rem;
        box-shadow: var(--shadow-popup);
    }

    /* 드롭 대상 섹션: 흐려지는 대신 뚜렷한 테두리 + 배경 강조로 표시 */
    .drag-over-section {
        outline: 2px dashed var(--accent-color);
        outline-offset: -2px;
        background-color: var(--button-hover-bg-color);
        opacity: 1;
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

    .result-file.text-hidden {
        color: var(--reverse-third-color);
        font-style: italic;
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
