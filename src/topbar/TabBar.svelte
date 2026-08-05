<script lang="ts">
  import { afterUpdate, onMount, onDestroy } from "svelte";
  import { ChevronDown } from "lucide-svelte";
  import { openTabs, relativeFilePath, selectedCursor, pushClosedTab, popClosedTab } from "../stores";
  import { registerAction, unregisterAction } from "../shortcut";

  let tabsEl: HTMLDivElement | null = null;
  let overflowing = false;
  let showList = false;

  // 파일이 열리면 탭 맨 앞에 등록 (이미 있으면 그 탭이 활성화될 뿐)
  $: {
    const p = $relativeFilePath;
    if (p && !$openTabs.includes(p)) {
      openTabs.update((t) => [p, ...t]);
    }
  }

  /** "/blog/my-post/_index.md" → "my-post", "/blog/foo.md" → "foo" */
  function displayName(path: string): string {
    const parts = path.split("/").filter(Boolean);
    if (parts[parts.length - 1] === "_index.md") {
      return parts[parts.length - 2] ?? path;
    }
    return (parts[parts.length - 1] ?? path).replace(/\.md$/, "");
  }

  function activate(path: string) {
    if (path === $relativeFilePath) return;
    // 사이드바 클릭과 동일한 순서로 스토어 갱신 (unsaved 다이얼로그 흐름 재사용)
    selectedCursor.set(path.endsWith("/_index.md") ? path.slice(0, -"/_index.md".length) : path);
    relativeFilePath.set(path);
  }

  function closeTab(path: string) {
    const tabs = $openTabs;
    const idx = tabs.indexOf(path);
    const next = tabs.filter((t) => t !== path);
    pushClosedTab(path); // Mod+Shift+T 복원용
    openTabs.set(next);
    if (path === $relativeFilePath) {
      const fallback = next[idx] ?? next[idx - 1];
      if (fallback) {
        activate(fallback);
      } else {
        relativeFilePath.set("");
        selectedCursor.set("");
      }
    }
  }

  function onAuxClick(e: MouseEvent, path: string) {
    if (e.button === 1) {
      e.preventDefault();
      closeTab(path);
    }
  }

  // 세로 휠로 탭 가로 스크롤 (스크롤바는 숨김)
  function onWheel(e: WheelEvent) {
    if (!tabsEl) return;
    const delta = Math.abs(e.deltaY) > Math.abs(e.deltaX) ? e.deltaY : e.deltaX;
    if (delta !== 0) {
      e.preventDefault();
      tabsEl.scrollLeft += delta;
    }
  }

  // 오버플로 감지 + 활성 탭이 바뀌면 보이는 위치로 스크롤
  let prevActive = "";
  afterUpdate(() => {
    if (!tabsEl) return;
    const ov = tabsEl.scrollWidth > tabsEl.clientWidth + 1;
    if (ov !== overflowing) overflowing = ov;
    if ($relativeFilePath !== prevActive) {
      prevActive = $relativeFilePath;
      tabsEl.querySelector<HTMLElement>(".tab.active")
        ?.scrollIntoView({ inline: "nearest", block: "nearest" });
    }
  });

  // Mod+Shift+T: 최근 닫은 탭 복원 (LIFO)
  onMount(() => {
    registerAction("reopen-tab", () => {
      const p = popClosedTab($openTabs);
      if (p) activate(p);
    });
  });
  onDestroy(() => {
    unregisterAction("reopen-tab");
  });

  function onWindowClick(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (!t?.closest?.(".tab-overflow")) showList = false;
  }
</script>

<svelte:window on:click={onWindowClick} />

<div class="tabbar-wrap">
  <div class="tabbar" bind:this={tabsEl} on:wheel|nonpassive={onWheel}>
    {#each $openTabs as tab (tab)}
      <div
        class="tab"
        class:active={tab === $relativeFilePath}
        role="tab"
        tabindex="0"
        aria-selected={tab === $relativeFilePath}
        title={tab}
        on:click={() => activate(tab)}
        on:keydown={(e) => { if (e.key === "Enter") activate(tab); }}
        on:auxclick={(e) => onAuxClick(e, tab)}
        on:mousedown={(e) => { if (e.button === 1) e.preventDefault(); }}
      >
        <span class="tab-name">{displayName(tab)}</span>
        <button class="tab-close" title="Close" on:click|stopPropagation={() => closeTab(tab)}>×</button>
      </div>
    {/each}
  </div>

  {#if overflowing || showList}
    <div class="tab-overflow">
      <button class="overflow-btn" title="All tabs" on:click={() => (showList = !showList)}>
        <ChevronDown size={14} />
      </button>
      {#if showList}
        <!-- 목록 안에서 닫기(× 또는 휠클릭)해도 드롭다운은 유지 —
             접힘은 ⌄ 재클릭 또는 바깥 클릭으로만 -->
        <div class="tab-list modal-surface" role="listbox">
          {#each $openTabs as tab (tab)}
            <div
              class="tab-list-item"
              class:active={tab === $relativeFilePath}
              on:auxclick={(e) => onAuxClick(e, tab)}
              on:mousedown={(e) => { if (e.button === 1) e.preventDefault(); }}
              role="option"
              aria-selected={tab === $relativeFilePath}
              tabindex="-1"
            >
              <button class="tab-list-name btn-plain hover-surface" title={tab}
                on:click={() => { activate(tab); showList = false; }}>{displayName(tab)}</button>
              <button class="tab-list-close btn-plain" title="Close" on:click={() => closeTab(tab)}>×</button>
            </div>
          {/each}
          {#if $openTabs.length === 0}
            <div class="tab-list-empty">No open tabs</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* TopBar 행 안에 내장 — 배경/테두리는 행이 담당 */
  .tabbar-wrap {
    position: relative;
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
  }

  .tabbar {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: stretch;
    overflow-x: auto;
    overflow-y: hidden;
    user-select: none;
    scrollbar-width: none; /* 스크롤바 숨김 — 휠/⋯ 버튼으로 탐색 */
  }
  .tabbar::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px 0 10px;
    max-width: 160px;
    min-width: 96px; /* 이름이 짧아도 탭이 너무 작아지지 않게 */
    flex-shrink: 0;
    font-size: 13px;
    color: var(--reverse-secondary-color);
    border-right: 1px solid var(--border-color);
    border-top: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
  }
  .tab:hover {
    background-color: var(--button-hover-bg-color);
  }
  .tab.active {
    background-color: var(--content-bg-color);
    color: var(--reverse-primary-color);
    border-top-color: var(--accent-color);
  }

  .tab-name {
    flex: 1;
    min-width: 0;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
  }

  .tab-close {
    flex-shrink: 0;
    border: none;
    background: none;
    box-shadow: none;
    padding: 0 3px;
    font-size: 13px;
    line-height: 1;
    border-radius: 3px;
    color: inherit;
    opacity: 0;
  }
  .tab:hover .tab-close,
  .tab.active .tab-close {
    opacity: 0.6;
  }
  .tab-close:hover {
    opacity: 1 !important;
    background-color: var(--tertiary-color);
  }

  /* 오버플로 시 전체 탭 목록 */
  .tab-overflow {
    position: relative;
    display: flex;
    align-items: center;
    padding: 0 3px;
    border-left: 1px solid var(--border-color);
    flex-shrink: 0;
  }
  .overflow-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    background: none;
    box-shadow: none;
    border-radius: 3px;
    color: var(--reverse-secondary-color);
    opacity: 0.7;
  }
  .overflow-btn:hover {
    opacity: 1;
    border: none;
    background-color: var(--button-hover-bg-color);
  }
  .tab-list {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 100;
    min-width: 190px;
    max-height: 50vh;
    overflow-y: auto;
    border-radius: 6px;
    padding: 4px;
    font-family: var(--font-ui);
  }
  .tab-list-item {
    display: flex;
    align-items: center;
  }
  .tab-list-item.active .tab-list-name {
    color: var(--accent-strong);
    font-weight: 700;
  }
  .tab-list-name {
    flex: 1;
    min-width: 0;
    text-align: left;
    padding: 3px 8px;
    font-size: 12px;
    border-radius: 3px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tab-list-close {
    flex-shrink: 0;
    padding: 0 6px;
    opacity: 0.5;
    border-radius: 3px;
  }
  .tab-list-close:hover {
    opacity: 1;
  }
  .tab-list-empty {
    padding: 4px 8px;
    font-size: 12px;
    opacity: 0.5;
  }
</style>
