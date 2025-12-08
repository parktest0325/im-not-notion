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

  function resolvePath(base: string, target: string): string {
    if (target.startsWith("/")) {
      base = "";
    }

    const parts = (base || "/").split("/");
    const segments = target.split("/");

    for (const seg of segments) {
      if (!seg || seg === ".") continue;
      if (seg === "..") {
        if (parts.length > 1) parts.pop();
      } else {
        parts.push(seg);
      }
    }

    let path = parts.join("/");
    if (!path.startsWith("/")) path = "/" + path;
    return path.replace(/\/+/g, "/");
  }

  onMount(async () => {
    try {
      const res: string = await invoke("execute_ssh", { cmd: "echo $HOME" });
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

    if (cmd.startsWith("cd ") || cmd === "cd") {
      const target = cmd.slice(2).trim();
      currentDir = resolvePath(currentDir, target || "/");
      if (output) {
        output += "\n";
      }
      output += `$ ${cmd}\n`;
      command = "";
      return;
    }

    try {
      const result: string = await invoke("execute_ssh", {
        cmd: `cd ${currentDir}; ${cmd} 2>&1`,
      });

      if (output) {
        output += "\n";
      }
      output += `$ ${cmd}\n`;
      if (result.trim()) {
        output += `${result.trimEnd()}\n`;
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
