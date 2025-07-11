<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { afterUpdate, onMount } from "svelte";
  export let show: boolean;
  export let closeTerminal: () => void;

  let command = "";
  let output = "";
  let currentDir = "";
  let terminalEl: HTMLPreElement | null = null;

  onMount(async () => {
    try {
      const res: string = await invoke("execute_ssh", { cmd: "pwd" });
      currentDir = res.trim();
    } catch (e) {
      console.error(e);
    }
  });

  async function sendCommand() {
    const cmd = command.trim();
    if (!cmd) return;
    if (cmd === "clear") {
      output = "";
      command = "";
      return;
    }
    try {
      const result: string = await invoke("execute_ssh", {
        cmd: `cd ${currentDir}; ${cmd} 2>&1; pwd`,
      });

      const trimmed = result.trimEnd();
      const idx = trimmed.lastIndexOf("\n");
      let newDir = currentDir;
      let cmdOutput = trimmed;
      if (idx !== -1) {
        cmdOutput = trimmed.slice(0, idx);
        newDir = trimmed.slice(idx + 1).trim();
      }

      if (output) {
        output += "\n";
      }
      output += `$ ${cmd}\n`;
      if (cmdOutput) {
        output += `${cmdOutput}\n`;
      }

      if (idx !== -1) {
        currentDir = newDir;
      }
    } catch (e) {
      if (output) {
        output += "\n";
      }
      output += `$ ${cmd}\nError: ${e}\n`;
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
    <div class="text-gray-400 mt-2">{currentDir}</div>
    <div class="flex mt-1 space-x-2">
      <input
        class="flex-grow p-2 rounded bg-gray-800 text-white"
        bind:value={command}
        on:keydown={(e) => {
          if (e.key === 'Enter') {
            e.stopPropagation();
            sendCommand();
          }
        }}
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
    max-width: 40rem;
  }
</style>
