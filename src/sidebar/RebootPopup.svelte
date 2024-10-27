<script lang="ts">
    import { invoke } from "@tauri-apps/api";
    export let show: boolean;
    export let closeReboot: () => void;

    async function handleReboot() {
        try {
            console.log("Rebooting the server...");
            await invoke("kill_server");
            await invoke("start_server");
        } catch (error) {
            console.log("Failed to reboot the server:", error);
        } finally {
            closeReboot();
        }
    }
</script>

{#if show}
    <div
        class="fixed inset-0 bg-black bg-opacity-50 flex justify-center items-center"
    >
        <div class="bg-white p-4 rounded-lg max-w-sm w-full">
            <h2 class="font-bold text-lg mb-4">Reboot Server</h2>
            <p>Are you sure you want to reboot the server?</p>
            <div class="flex justify-end space-x-2 mt-4">
                <button
                    class="bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded"
                    on:click={handleReboot}>Yes</button
                >
                <button
                    class="bg-gray-500 hover:bg-gray-700 text-white font-bold py-2 px-4 rounded"
                    on:click={closeReboot}>No</button
                >
            </div>
        </div>
    </div>
{/if}
