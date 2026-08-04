<script lang="ts">
  import { isConnected, activeServerName, lastSavedAt } from "../stores";

  function fmtTime(d: Date): string {
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
</script>

<div class="statusbar">
  <span class="item">
    <span
      class="dot"
      class:pulse={$isConnected}
      style="background-color: {$isConnected
        ? 'var(--status-connected-color)'
        : 'var(--status-disconnected-color)'}"
    ></span>
    {$isConnected ? "Connected" : "Not Connected"}{$activeServerName ? ` · ${$activeServerName}` : ""}
  </span>
  <!-- 파일 경로는 TopBar 브레드크럼에 표시 -->
  {#if $lastSavedAt}
    <span class="item saved">Saved {fmtTime($lastSavedAt)}</span>
  {/if}
</div>

<style>
  .statusbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    height: 24px;
    padding: 0 0.75rem;
    font-size: 11px;
    background-color: var(--topbar-bg-color);
    border-top: 1px solid var(--border-color);
    color: var(--reverse-secondary-color);
    flex-shrink: 0;
    user-select: none;
  }
  .item {
    display: flex;
    align-items: center;
    white-space: nowrap;
  }
  .saved {
    margin-left: auto;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 9999px;
    display: inline-block;
    margin-right: 6px;
    flex-shrink: 0;
  }
  .dot.pulse {
    animation: status-pulse 2s ease-in-out infinite;
  }
  @keyframes status-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.45; }
  }
</style>
