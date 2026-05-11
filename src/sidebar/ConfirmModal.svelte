<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let title: string = "Confirm";
  export let message: string = "";
  export let confirmLabel: string = "OK";
  export let cancelLabel: string = "Cancel";
  export let danger: boolean = false;

  const dispatch = createEventDispatcher<{ confirm: void; cancel: void }>();

  function onConfirm() { dispatch("confirm"); }
  function onCancel() { dispatch("cancel"); }
</script>

<div class="fixed inset-0 bg-black/70 z-[65] flex items-center justify-center" role="dialog">
  <div class="bg-zinc-900 border border-zinc-700 rounded-lg w-[400px] p-5">
    <h3 class="text-sm font-semibold mb-2">{title}</h3>
    <p class="text-xs text-zinc-300 mb-4 whitespace-pre-line">{message}</p>
    <div class="flex gap-2 justify-end">
      <button
        class="px-3 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-xs"
        on:click={onCancel}
      >
        {cancelLabel}
      </button>
      <button
        class="px-3 py-1 rounded text-xs font-medium {danger ? 'bg-red-700 hover:bg-red-600' : 'bg-blue-700 hover:bg-blue-600'}"
        on:click={onConfirm}
      >
        {confirmLabel}
      </button>
    </div>
  </div>
</div>
