<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { afterUpdate } from "svelte";
  export let show: boolean;
  export let closeTerminal: () => void;

  let command = "";
  let output = "";
  let terminalEl: HTMLPreElement | null = null;

  async function sendCommand() {
    const cmd = command.trim();
    if (!cmd) return;
    if (cmd === "clear") {
      output = "";
    } else {
      try {
        const result: string = await invoke("execute_ssh", { cmd });
        output += `$ ${cmd}\n${result}\n`;
      } catch (e) {
        output += `$ ${cmd}\nError: ${e}\n`;
      }
    }
    command = "";
  }
  afterUpdate(() => {
    if (terminalEl) {
      terminalEl.scrollTop = terminalEl.scrollHeight;
    }
  });
</script>

<div class="terminal-popup">
  <Popup {show} closePopup={closeTerminal}>
    <h2 class="font-bold text-lg mb-2">SSH Terminal</h2>
    <pre class="terminal-output" bind:this={terminalEl}>{output}</pre>
    <div class="flex mt-2 space-x-2">
      <input
        class="flex-grow p-2 rounded bg-gray-800 text-white"
        bind:value={command}
        on:keydown={(e) => e.key === 'Enter' && sendCommand()}
        placeholder="Enter command" />
      <button class="bg-blue-600 hover:bg-blue-800 px-4 rounded" on:click={sendCommand}>Run</button>
    </div>
  </Popup>
</div>

<style>
  .terminal-output {
    background-color: #000;
    color: #0f0;
    padding: 0.5rem;
    height: 15rem;
    overflow-y: auto;
    white-space: pre-wrap;
  }

  :global(.terminal-popup .popup-content) {
    max-width: 48rem;
  }
</style>
