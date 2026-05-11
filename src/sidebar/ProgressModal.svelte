<script lang="ts">
  import type { TransferProgress } from "../types/explorer";

  export let progress: TransferProgress | null;

  $: pct = progress && progress.total_bytes > 0
    ? Math.min(100, (progress.current_bytes / progress.total_bytes) * 100)
    : 0;

  $: filePct = progress && progress.files_total > 0
    ? (progress.files_done / progress.files_total) * 100
    : 0;

  function phaseLabel(phase: string): string {
    switch (phase) {
      case "packing": return "Packing";
      case "uploading": return "Uploading";
      case "extracting": return "Extracting";
      case "downloading": return "Downloading";
      case "cleanup": return "Cleaning up";
      case "done": return "Done";
      case "error": return "Error";
      default: return phase;
    }
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }
</script>

{#if progress}
<div class="fixed inset-0 bg-black/70 z-[70] flex items-center justify-center" role="dialog">
  <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-[420px] p-5">
    <h3 class="text-sm font-semibold mb-3">
      {phaseLabel(progress.phase)}
    </h3>

    {#if progress.phase === "error"}
      <p class="text-red-400 text-xs mb-3">{progress.error ?? "Unknown error"}</p>
    {/if}

    <div class="mb-2">
      <div class="flex justify-between text-xs text-zinc-400 mb-1">
        <span class="truncate flex-1 mr-2">{progress.current_file || ""}</span>
        <span>{Math.round(pct)}%</span>
      </div>
      <div class="h-2 bg-zinc-800 rounded overflow-hidden">
        <div class="h-full bg-blue-500 transition-all" style="width: {pct}%"></div>
      </div>
      {#if progress.total_bytes > 0}
        <div class="text-[10px] text-zinc-500 mt-1">
          {fmtSize(progress.current_bytes)} / {fmtSize(progress.total_bytes)}
        </div>
      {/if}
    </div>

    {#if progress.files_total > 0}
      <div class="text-xs text-zinc-400">
        Files {progress.files_done} / {progress.files_total}
      </div>
      <div class="h-1 bg-zinc-800 rounded overflow-hidden mt-1">
        <div class="h-full bg-emerald-500 transition-all" style="width: {filePct}%"></div>
      </div>
    {/if}
  </div>
</div>
{/if}
