<script lang="ts">
  export let isMenuOpen: boolean;
  export let toggleMenu: () => void;
  import { PanelLeftOpen, ExternalLink, ChevronRight } from "lucide-svelte";
  import { relativeFilePath, url, hiddenPath, fullFilePath, addToast, isEditingContent } from "../stores";
  import { type GlobalFunctions, GLOBAL_FUNCTIONS } from "../context";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-shell";
  import { getContext } from "svelte";
  import TabBar from "./TabBar.svelte";

  let isHidden = false;
  let isLoading = false;

  const { refreshList } = getContext<GlobalFunctions>(GLOBAL_FUNCTIONS);

  function handleOpenPage() {
    let cleanedPath = $fullFilePath
      .replace(/\.md$/, "")
      .replace(/\/_index$/, "")
      .toLowerCase();

    try {
      const fullUrl = new URL(cleanedPath, $url);
      open(fullUrl.toString().toLowerCase());
    } catch (error) {
      console.error("Failed to open page:", error);
      addToast("Invalid blog URL. Check the url in settings.");
    }
  }

  async function checkHidden() {
    try {
      isHidden = await invoke("check_file_hidden", { path: $relativeFilePath });
    } catch (error) {
      console.error("Failed to check hidden status:", error);
      isHidden = false;
      addToast("Failed to check hidden status.");
    }
  }

  async function toggleHidden() {
    if (!$relativeFilePath || isLoading) return;

    isLoading = true;
    try {
      await invoke("toggle_hidden_file", { path: $relativeFilePath, state: isHidden });
      isHidden = !isHidden;
      await refreshList();
      addToast(isHidden ? "File hidden." : "File visible.", "success");
    } catch (error) {
      console.error("Failed to toggle hidden status:", error);
      addToast("Failed to toggle hidden status.");
    } finally {
      isLoading = false;
    }
  }

  $: if ($relativeFilePath) {
    // 파일 선택 시 숨김 상태를 확인하고 전체 경로를 설정
    checkHidden();

    // relativeFilePath가 갱신되면 전체 파일 경로 갱신
    // relativeFilePath에 섹션이 포함됨: e.g. "/posts/my-post/_index.md"
    const newPath = (isHidden ? `/${$hiddenPath}` : '') + $relativeFilePath;
    fullFilePath.set(newPath);
  }
</script>

<!-- 메인 행: [사이드바 토글] [탭들 ...] [Hide/Show] [브라우저 열기] -->
<div class="topbar-row">
  {#if !isMenuOpen}
    <button class="icon-action" on:click={toggleMenu} title="Open sidebar">
      <PanelLeftOpen size={15} />
    </button>
  {/if}
  <div class="tabs-host"><TabBar /></div>
  <div class="actions">
    {#if $relativeFilePath && !$relativeFilePath.endsWith('_index.md')}
      <button
        on:click={toggleHidden}
        class="vis-btn"
        class:btn-visible={!isHidden}
        class:btn-hidden={isHidden}
      >
        {isHidden ? "Show" : "Hide"}
      </button>
    {/if}
    {#if $relativeFilePath}
      <button class="icon-action" on:click={handleOpenPage} title="Open in browser">
        <ExternalLink size={14} />
      </button>
    {/if}
  </div>
</div>

<!-- 브레드크럼: 파일이 열려 있을 때만 나오는 얇은 줄 -->
{#if $fullFilePath}
  {@const crumbs = $fullFilePath.split("/").filter(Boolean)}
  <div class="crumbbar" class:editing={$isEditingContent}>
    {#each crumbs as crumb, i}
      {#if i > 0}
        <span class="crumb-sep"><ChevronRight size={11} /></span>
      {/if}
      <span class="crumb" class:last={i === crumbs.length - 1}>{crumb}</span>
    {/each}
    {#if $isEditingContent}
      <span class="edit-badge">EDIT</span>
    {/if}
  </div>
{/if}

<style>
  .topbar-row {
    display: flex;
    align-items: stretch;
    height: 34px;
    flex-shrink: 0;
    background-color: var(--secondary-color);
    border-bottom: 1px solid var(--border-color);
  }

  .tabs-host {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: stretch;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    flex-shrink: 0;
  }

  .icon-action {
    display: flex;
    align-items: center;
    justify-content: center;
    align-self: center;
    width: 26px;
    height: 26px;
    margin: 0 2px;
    padding: 0;
    border: none;
    background: none;
    box-shadow: none;
    border-radius: 0.25rem;
    color: var(--reverse-secondary-color);
    opacity: 0.75;
  }
  .icon-action:hover {
    opacity: 1;
    background-color: var(--button-hover-bg-color);
    border: none;
  }

  /* 크롬 요소는 각지게 (3px) — 콘텐츠 UI와 톤 일치 */
  .vis-btn {
    padding: 2px 10px;
    font-size: 12px;
    border-radius: 3px;
    border: 1px solid transparent;
    box-shadow: none;
    transition: background-color 0.15s;
  }

  /* 브레드크럼 줄: 에디터 배경 위에 얇게 */
  .crumbbar {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 26px;
    padding: 0 14px;
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--reverse-secondary-color);
    background-color: var(--content-bg-color);
    border-bottom: 1px solid var(--border-color); /* 콘텐츠 영역과의 경계 */
    overflow: hidden;
    white-space: nowrap;
    user-select: none;
  }
  .crumb {
    opacity: 0.7;
  }
  .crumb.last {
    opacity: 1;
    font-weight: 700;
    color: var(--reverse-primary-color);
  }
  .crumb-sep {
    display: flex;
    align-items: center;
    opacity: 0.4;
    flex-shrink: 0;
  }

  /* 에디트 모드 표시: 브레드크럼 하단 액센트 라인 + EDIT 배지 */
  .crumbbar.editing {
    box-shadow: inset 0 -2px 0 var(--accent-color);
  }
  .edit-badge {
    margin-left: auto;
    padding: 0 6px;
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.05em;
    line-height: 1.6;
    border-radius: 3px;
    background-color: var(--accent-tint);
    color: var(--accent-strong);
    flex-shrink: 0;
  }

  .btn-visible {
    background-color: var(--btn-visible-bg);
    color: var(--btn-visible-text);
    border-color: var(--btn-visible-border);
  }
  .btn-visible:hover {
    background-color: var(--btn-visible-hover-bg);
  }
  .btn-hidden {
    background-color: var(--btn-hidden-bg);
    color: var(--btn-hidden-text);
    border-color: var(--btn-hidden-border);
  }
  .btn-hidden:hover {
    background-color: var(--btn-hidden-hover-bg);
  }
</style>
