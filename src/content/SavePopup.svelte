<script lang="ts">
    import Popup from "../component/Popup.svelte";
    export let show: boolean;
    export let closeSave: () => void;
    export let handleSave: () => void;
  
</script>
  
<Popup {show} closePopup={closeSave}>
    <h2 class="font-bold text-lg mb-4">Save Content</h2>
    <p>Are you Saving?</p>
    <div class="flex justify-end space-x-2 mt-4">
        <button
        class="btn-confirm font-bold py-2 px-4 rounded"
        on:click={handleSave}>Yes</button>
        <button
        class="btn-cancel font-bold py-2 px-4 rounded"
        on:click={closeSave}>No</button>
    </div>
</Popup>

<style>
  .btn-confirm { background-color: var(--btn-confirm-bg); color: var(--btn-confirm-text); }
  .btn-confirm:hover { background-color: var(--btn-confirm-hover-bg); }
  .btn-cancel { background-color: var(--btn-cancel-bg); color: var(--btn-cancel-text); }
  .btn-cancel:hover { background-color: var(--btn-cancel-hover-bg); }
</style>
