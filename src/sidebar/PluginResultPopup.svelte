<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { addToast } from "../stores";
  import type { ResultPage } from "../types/generated";

  export let show: boolean;
  export let title: string = "";
  export let body: string = "";
  export let pages: ResultPage[] = [];
  export let onClose: () => void;

  // Resolve pages: if pages array provided use it, else single page from body
  let resolvedPages: ResultPage[] = [];
  let currentPage: number = 0;

  $: {
    if (pages && pages.length > 0) {
      resolvedPages = pages;
    } else {
      resolvedPages = [{ title: "", body: body ?? "" }];
    }
    currentPage = 0;
  }

  // Parse body into segments: text and copy blocks
  type Segment = { type: "text"; content: string } | { type: "copy"; title: string; content: string };

  function parseBody(raw: string): Segment[] {
    const segments: Segment[] = [];
    const regex = /\{\{copy:(.+?)\}\}\n?([\s\S]*?)\n?\{\{\/copy\}\}/g;
    let lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = regex.exec(raw)) !== null) {
      if (match.index > lastIndex) {
        segments.push({ type: "text", content: raw.slice(lastIndex, match.index) });
      }
      segments.push({ type: "copy", title: match[1].trim(), content: match[2] });
      lastIndex = regex.lastIndex;
    }

    if (lastIndex < raw.length) {
      segments.push({ type: "text", content: raw.slice(lastIndex) });
    }

    return segments;
  }

  $: currentBody = resolvedPages[currentPage]?.body ?? "";
  $: segments = parseBody(currentBody);

  let openBlocks: Record<string, boolean> = {};

  function toggleBlock(blockTitle: string) {
    openBlocks[blockTitle] = !openBlocks[blockTitle];
    openBlocks = openBlocks;
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      addToast("Copied.", "success");
    } catch (_) {
      addToast("Copy failed.");
    }
  }

  async function copyCurrentPage() {
    await copyText(currentBody);
  }
</script>

<Popup {show} closePopup={onClose}>
  <div class="flex justify-between items-center">
    <h3 class="text-lg font-bold">{title}</h3>
    <button class="copy-btn" title="Copy page" on:click={copyCurrentPage}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
      </svg>
    </button>
  </div>

  {#if resolvedPages.length > 1}
    <div class="tab-bar">
      {#each resolvedPages as page, i}
        <button
          class="tab-btn"
          class:tab-active={currentPage === i}
          on:click={() => { currentPage = i; openBlocks = {}; }}
        >{page.title}</button>
      {/each}
    </div>
  {/if}

  <div class="result-body">
    {#each segments as seg}
      {#if seg.type === "text"}
        <pre class="result-pre">{seg.content}</pre>
      {:else}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div class="copy-block">
          <div class="copy-block-header" on:click={() => toggleBlock(seg.title)}>
            <span class="copy-block-arrow">{openBlocks[seg.title] ? "\u25BC" : "\u25B6"}</span>
            <span class="copy-block-title">{seg.title}</span>
            <button
              class="copy-btn copy-btn-sm"
              title="Copy"
              on:click|stopPropagation={() => copyText(seg.content)}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
            </button>
          </div>
          {#if openBlocks[seg.title]}
            <pre class="copy-block-body">{seg.content}</pre>
          {/if}
        </div>
      {/if}
    {/each}
  </div>

  <button
    class="w-full p-2 rounded mt-2 opacity-60"
    on:click={onClose}
  >Close</button>
</Popup>

<style>
  .result-body {
    max-height: 60vh;
    overflow: auto;
    border-radius: 0.375rem;
    padding: 0.75rem;
    background-color: var(--sidebar-bg-color);
    border: 1px solid var(--border-color);
  }
  .result-pre {
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.8rem;
    line-height: 1.5;
    margin: 0;
  }
  .copy-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 0.25rem;
    border: 1px solid var(--border-color);
    background: none;
    cursor: pointer;
    opacity: 0.6;
  }
  .copy-btn:hover {
    opacity: 1;
    background-color: var(--button-active-bg-color);
  }
  .copy-btn-sm {
    width: 22px;
    height: 22px;
  }

  /* Tabs */
  .tab-bar {
    display: flex;
    gap: 0.25rem;
    border-bottom: 1px solid var(--border-color);
    margin-bottom: 0.5rem;
  }
  .tab-btn {
    padding: 0.3rem 0.75rem;
    font-size: 0.8rem;
    border: none;
    background: none;
    cursor: pointer;
    opacity: 0.5;
    border-bottom: 2px solid transparent;
  }
  .tab-btn:hover {
    opacity: 0.8;
  }
  .tab-active {
    opacity: 1;
    border-bottom-color: var(--button-active-bg-color);
  }

  /* Copy blocks */
  .copy-block {
    margin: 0.4rem 0;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    overflow: hidden;
  }
  .copy-block-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.5rem;
    cursor: pointer;
    background-color: var(--popup-bg-color);
    font-size: 0.8rem;
  }
  .copy-block-header:hover {
    background-color: var(--button-active-bg-color);
  }
  .copy-block-arrow {
    font-size: 0.65rem;
    width: 0.8rem;
    flex-shrink: 0;
  }
  .copy-block-title {
    flex: 1;
    font-weight: 500;
  }
  .copy-block-body {
    white-space: pre-wrap;
    word-break: break-all;
    font-size: 0.78rem;
    line-height: 1.5;
    margin: 0;
    padding: 0.5rem;
    border-top: 1px solid var(--border-color);
    background-color: var(--sidebar-bg-color);
  }
</style>
