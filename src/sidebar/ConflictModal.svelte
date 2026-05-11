<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { ConflictItem, ConflictPolicy } from "../types/explorer";

  export let conflicts: ConflictItem[];

  const dispatch = createEventDispatcher<{
    resolve: { policy: ConflictPolicy };
    cancel: void;
  }>();

  function pick(policy: ConflictPolicy) {
    dispatch("resolve", { policy });
  }

  function cancel() {
    dispatch("cancel");
  }

  function fmtSize(bytes: number, isDir: boolean): string {
    if (isDir) return "folder";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }
</script>

<div class="fixed inset-0 bg-black/70 z-[60] flex items-center justify-center" role="dialog">
  <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-[480px] max-h-[80vh] flex flex-col">
    <div class="px-4 py-3 border-b border-zinc-700">
      <h3 class="text-sm font-semibold text-yellow-400">⚠ File conflicts ({conflicts.length})</h3>
      <p class="text-xs text-zinc-400 mt-1">Items with the same name already exist at the destination.</p>
    </div>

    <div class="overflow-auto p-3 max-h-60">
      <table class="w-full text-xs">
        <tbody>
          {#each conflicts as c}
            <tr class="border-b border-zinc-800 last:border-0">
              <td class="py-1 truncate">
                <span class="mr-1">{c.is_dir ? "📁" : "📄"}</span>
                {c.name}
              </td>
              <td class="py-1 text-right text-zinc-500">{fmtSize(c.size, c.is_dir)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="px-4 py-3 border-t border-zinc-700 flex gap-2">
      <button
        class="flex-1 px-3 py-2 bg-red-700 hover:bg-red-600 rounded text-xs font-medium"
        on:click={() => pick("overwrite")}
      >
        Overwrite all
      </button>
      <button
        class="flex-1 px-3 py-2 bg-zinc-700 hover:bg-zinc-600 rounded text-xs font-medium"
        on:click={() => pick("skip")}
      >
        Skip all
      </button>
      <button
        class="flex-1 px-3 py-2 bg-blue-700 hover:bg-blue-600 rounded text-xs font-medium"
        on:click={() => pick("rename")}
      >
        Rename
      </button>
      <button
        class="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 rounded text-xs"
        on:click={cancel}
      >
        Cancel
      </button>
    </div>
  </div>
</div>
