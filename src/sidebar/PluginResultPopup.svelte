<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { addToast } from "../stores";

  export let show: boolean;
  export let title: string = "";
  export let body: string = "";
  export let onClose: () => void;

  async function copyBody() {
    try {
      await navigator.clipboard.writeText(body);
      addToast("Copied.", "success");
    } catch (_) {
      addToast("Copy failed.");
    }
  }
</script>

<Popup {show} closePopup={onClose}>
  <div class="flex justify-between items-center">
    <h3 class="text-lg font-bold">{title}</h3>
    <button class="copy-btn" title="Copy" on:click={copyBody}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
      </svg>
    </button>
  </div>
  <div class="result-body">
    <pre>{body}</pre>
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
  .result-body pre {
    white-space: pre;
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
</style>
