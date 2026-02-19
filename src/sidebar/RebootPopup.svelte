<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { addToast } from "../stores";
  export let show: boolean;
  export let closeReboot: () => void;

  async function handleReboot() {
    try {
      console.log("Rebooting the server...");
      await invoke("kill_server");
      await invoke("start_server");
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
      class="bg-red-600 hover:bg-red-800 font-bold py-2 px-4 rounded"
      on:click={handleReboot}>Yes</button>
    <button
      class="bg-gray-600 hover:bg-gray-800 font-bold py-2 px-4 rounded"
      on:click={closeReboot}>No</button>
  </div>
</Popup>