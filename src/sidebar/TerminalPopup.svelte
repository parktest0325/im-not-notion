<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { afterUpdate, onMount, tick } from "svelte";
  export let show: boolean;
  export let closeTerminal: () => void;

  type LineType = "cmd" | "output" | "error";
  interface TermLine { type: LineType; text: string; }

  let command = "";
  let lines: TermLine[] = [];
  let currentDir = "";
  let terminalEl: HTMLDivElement | null = null;
  let inputEl: HTMLInputElement | null = null;

  // 커맨드 히스토리
  let commandHistory: string[] = [];
  let historyIndex = -1;
  let savedCommand = "";

  onMount(async () => {
    try {
      const res: string = await invoke("execute_ssh", { cmd: "echo $HOME" });
      currentDir = res.trim();
    } catch (e) {
      console.error(e);
    }
  });

  // 팝업 열릴 때 입력창 포커스
  $: if (show) {
    tick().then(() => inputEl?.focus());
  }

  function pushLine(type: LineType, text: string) {
    lines = [...lines, { type, text }];
  }

  async function sendCommand() {
    const cmd = command.trim();
    if (!cmd) return;

    // 히스토리에 추가 (중복 방지)
    if (commandHistory[commandHistory.length - 1] !== cmd) {
      commandHistory = [...commandHistory, cmd];
    }
    historyIndex = -1;
    savedCommand = "";

    // clear 명령
    if (cmd === "clear") {
      lines = [];
      command = "";
      return;
    }

    // cd 명령 — 서버에서 검증
    if (cmd === "cd" || cmd.startsWith("cd ")) {
      const target = cmd.slice(2).trim();
      const cdCmd = target ? `cd ${target}` : "cd";
      pushLine("cmd", `[${currentDir}]$ ${cmd}`);
      try {
        const result: string = await invoke("execute_ssh", {
          cmd: `cd ${currentDir} && ${cdCmd} && pwd 2>&1`,
        });
        const newDir = result.trim();
        if (newDir.startsWith("/")) {
          currentDir = newDir;
        } else {
          pushLine("error", newDir);
        }
      } catch (e) {
        pushLine("error", `${e}`);
      }
      command = "";
      return;
    }

    // 일반 명령
    pushLine("cmd", `[${currentDir}]$ ${cmd}`);
    try {
      const result: string = await invoke("execute_ssh", {
        cmd: `cd ${currentDir}; ${cmd} 2>&1`,
      });
      if (result.trim()) {
        pushLine("output", result.trimEnd());
      }
    } catch (e) {
      pushLine("error", `Error: ${e}`);
    }
    command = "";
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.stopPropagation();
      sendCommand();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (commandHistory.length === 0) return;
      if (historyIndex === -1) {
        savedCommand = command;
        historyIndex = commandHistory.length - 1;
      } else if (historyIndex > 0) {
        historyIndex--;
      }
      command = commandHistory[historyIndex];
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (historyIndex === -1) return;
      if (historyIndex < commandHistory.length - 1) {
        historyIndex++;
        command = commandHistory[historyIndex];
      } else {
        historyIndex = -1;
        command = savedCommand;
      }
    }
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
    <div class="terminal-output" bind:this={terminalEl}>
      {#each lines as line}
        <pre class="term-line {line.type}">{line.text}</pre>
      {/each}
    </div>
    <div class="flex mt-1 space-x-2 items-center">
      <span class="text-gray-400 text-sm whitespace-nowrap">{currentDir}$</span>
      <input
        bind:this={inputEl}
        class="flex-grow p-2 rounded bg-gray-800 text-white font-mono"
        bind:value={command}
        on:keydown={onKeyDown}
        placeholder="Enter command" />
      <button class="bg-blue-600 hover:bg-blue-800 px-4 rounded whitespace-nowrap" on:click={sendCommand}>Run</button>
    </div>
  </Popup>
</div>

<style>
  .terminal-output {
    background-color: #000;
    padding: 0.5rem;
    height: 60vh;
    overflow-y: auto;
    font-family: monospace;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .term-line {
    margin: 0;
    padding: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .term-line.cmd {
    color: #5fd7ff; /* cyan — 명령어 입력 */
  }

  .term-line.output {
    color: #d4d4d4; /* light gray — 결과 출력 */
  }

  .term-line.error {
    color: #ff5f5f; /* red — 에러 */
  }

  :global(.terminal-popup .popup-content) {
    max-width: 56rem;
    max-height: 90vh;
  }
</style>
