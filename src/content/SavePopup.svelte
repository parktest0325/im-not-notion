<script lang="ts">
    import Popup from "../component/Popup.svelte";
    export let show: boolean;
    export let closeSave: () => void;
    export let handleSave: () => void;
    export let handleDiscard: (() => void) | null = null;
    export let title: string = "Save Content";
    export let message: string = "Are you Saving?";
    export let saveLabel: string = "Yes";
    export let discardLabel: string = "Discard";
    export let cancelLabel: string = "No";

</script>

<Popup {show} closePopup={closeSave}>
    <h2 class="font-bold text-lg mb-4">{title}</h2>
    <p>{message}</p>
    <div class="flex justify-end space-x-2 mt-4">
        <button
        class="btn-confirm font-bold py-2 px-4 rounded"
        on:click={handleSave}>{saveLabel}</button>
        {#if handleDiscard}
        <button
        class="btn-danger font-bold py-2 px-4 rounded"
        on:click={handleDiscard}>{discardLabel}</button>
        {/if}
        <button
        class="btn-cancel font-bold py-2 px-4 rounded"
        on:click={closeSave}>{cancelLabel}</button>
    </div>
</Popup>

<style>
  .btn-confirm { background-color: var(--btn-confirm-bg); color: var(--btn-confirm-text); }
  .btn-confirm:hover { background-color: var(--btn-confirm-hover-bg); }
  .btn-danger { background-color: var(--btn-danger-bg); color: var(--btn-danger-text); }
  .btn-danger:hover { background-color: var(--btn-danger-hover-bg); }
  .btn-cancel { background-color: var(--btn-cancel-bg); color: var(--btn-cancel-text); }
  .btn-cancel:hover { background-color: var(--btn-cancel-hover-bg); }
</style>
