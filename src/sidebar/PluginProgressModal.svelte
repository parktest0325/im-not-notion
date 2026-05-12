<script lang="ts">
  import type { PluginProgress } from "../types/generated";

  export let progress: PluginProgress | null;

  $: pct = progress && progress.total != null && progress.total > 0 && progress.current != null
    ? Math.min(100, (progress.current / progress.total) * 100)
    : null;
</script>

{#if progress}
<div class="fixed inset-0 bg-black/70 flex items-center justify-center" style="z-index: 1200;" role="dialog">
  <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-[440px] p-5">
    <h3 class="text-sm font-semibold mb-1">{progress.plugin}</h3>
    {#if progress.phase}
      <p class="text-xs text-zinc-400 mb-3">{progress.phase}</p>
    {/if}

    {#if progress.message}
      <p class="text-xs text-zinc-300 mb-3 whitespace-pre-line">{progress.message}</p>
    {/if}

    {#if pct != null}
      <div class="flex justify-between text-[10px] text-zinc-500 mb-1">
        <span>{progress.current?.toFixed(0)} / {progress.total?.toFixed(0)}</span>
        <span>{Math.round(pct)}%</span>
      </div>
      <div class="h-2 bg-zinc-800 rounded overflow-hidden">
        <div class="h-full bg-blue-500 transition-all" style="width: {pct}%"></div>
      </div>
    {:else}
      <div class="h-1 bg-zinc-800 rounded overflow-hidden">
        <div class="h-full bg-blue-500 animate-pulse" style="width: 33%"></div>
      </div>
    {/if}
  </div>
</div>
{/if}
