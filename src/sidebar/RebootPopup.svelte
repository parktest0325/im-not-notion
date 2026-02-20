<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../stores";
  export let show: boolean;
  export let closeReboot: () => void;

  async function handleReboot() {
    try {
      await invoke("kill_server");
      await invoke("start_server");
      addToast("Server rebooted.", "success");
    } catch (error) {
      console.error("Failed to reboot the server:", error);
      addToast("Failed to reboot server.");
    } finally {
      closeReboot();
    }
  }
</script>

<Popup {show} closePopup={closeReboot}>
  <h2 class="font-bold text-lg mb-4">Reboot Server</h2>
  <p>Are you sure you want to reboot the server?</p>
  <div class="flex justify-end space-x-2 mt-4">
    <button
      class="btn-danger font-bold py-2 px-4 rounded"
      on:click={handleReboot}>Yes</button>
    <button
      class="btn-cancel font-bold py-2 px-4 rounded"
      on:click={closeReboot}>No</button>
  </div>
</Popup>

<style>
  .btn-danger { background-color: var(--btn-danger-bg); color: var(--btn-danger-text); }
  .btn-danger:hover { background-color: var(--btn-danger-hover-bg); }
  .btn-cancel { background-color: var(--btn-cancel-bg); color: var(--btn-cancel-text); }
  .btn-cancel:hover { background-color: var(--btn-cancel-hover-bg); }
</style>