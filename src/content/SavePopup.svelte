<script lang="ts">
    import Popup from "../component/Popup.svelte";
    import { invoke } from "@tauri-apps/api";
    export let show: boolean;
    export let closeSave: () => void;
    export let handleSave: () => void;
  
</script>
  
<Popup {show} closePopup={closeSave}>
    <h2 class="font-bold text-lg mb-4">Save Content</h2>
    <p>Are you Saving?</p>
    <div class="flex justify-end space-x-2 mt-4">
        <button
        class="bg-blue-600 hover:bg-blue-800 font-bold py-2 px-4 rounded"
        on:click={handleSave}>Yes</button>
        <button
        class="bg-gray-600 hover:bg-gray-800 font-bold py-2 px-4 rounded"
        on:click={closeSave}>No</button>
    </div>
</Popup>
