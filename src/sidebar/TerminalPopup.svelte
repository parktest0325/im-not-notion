<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { onDestroy, tick } from "svelte";
  import { addToast } from "../stores";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { TerminalInputController } from "./terminal/TerminalInputController";
  import "@xterm/xterm/css/xterm.css";

  export let show: boolean;
  export let closeTerminal: () => void;

  let termContainer: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let inputController: TerminalInputController | null = null;
  let started = false;
  let fontSize = 14;

  function changeFontSize(delta: number) {
    fontSize = Math.max(8, Math.min(32, fontSize + delta));
    if (terminal) {
      terminal.options.fontSize = fontSize;
      fitAddon?.fit();
    }
  }

  function handleTerminalKey(e: KeyboardEvent) {
    if (!show || !started) return;
    if (e.ctrlKey && (e.key === "=" || e.key === "+")) {
      e.preventDefault();
      changeFontSize(1);
    } else if (e.ctrlKey && e.key === "-") {
      e.preventDefault();
      changeFontSize(-1);
    } else if (e.ctrlKey && e.key === "0") {
      e.preventDefault();
      changeFontSize(14 - fontSize);
    }
  }

  async function startTerminal() {
    if (started) return;
    await tick();
    if (!termContainer) return;

    const cs = getComputedStyle(document.documentElement);
    terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
      theme: {
        background: cs.getPropertyValue("--terminal-bg").trim() || "#1e1e1e",
        foreground: cs.getPropertyValue("--terminal-fg").trim() || "#d4d4d4",
        cursor: cs.getPropertyValue("--terminal-cursor").trim() || "#d4d4d4",
      },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(termContainer);
    fitAddon.fit();
    terminal.focus();

    inputController = new TerminalInputController(terminal, (data: string) => {
      return invoke("write_pty_cmd", { data });
    });
    inputController.attach();

    const { cols, rows } = terminal;

    const onEvent = new Channel<string>();
    onEvent.onmessage = (data: string) => {
      if (data === "\x00__PTY_CLOSED__") {
        stopTerminal();
        if (show) closeTerminal();
        return;
      }
      inputController?.handleOutput(data);
      terminal?.write(data);
    };

    try {
      await invoke("start_pty_cmd", { cols, rows, onEvent });
      started = true;
    } catch (e) {
      terminal.write(`\r\nError: ${e}\r\n`);
      addToast("Failed to connect terminal.");
      return;
    }

    terminal.onResize(({ cols, rows }: { cols: number; rows: number }) => {
      invoke("resize_pty_cmd", { cols, rows }).catch((e: unknown) => {
        console.error("resize_pty_cmd error:", e);
      });
    });
  }

  function stopTerminal() {
    if (started) {
      invoke("stop_pty_cmd").catch((e: unknown) => {
        console.error("stop_pty_cmd error:", e);
      });
    }
    inputController?.dispose();
    inputController = null;
    terminal?.dispose();
    terminal = null;
    fitAddon = null;
    started = false;
  }

  /** 닫기 버튼: Ctrl+D(EOF) 전송 → 정상 종료 시도 → 폴백으로 강제 종료 */
  function handleCloseBtn() {
    if (started) {
      invoke("write_pty_cmd", { data: "\x04" }).catch(() => {});
      // 셸이 EOF로 종료하면 __PTY_CLOSED__ 시그널이 와서 자동 정리됨.
      // 1초 안에 종료 안 되면 (vim 등) 강제 종료.
      setTimeout(() => {
        if (started) stopTerminal();
        if (show) closeTerminal();
      }, 1000);
    } else {
      closeTerminal();
    }
  }

  $: if (show) {
    startTerminal();
  } else if (!show && started) {
    stopTerminal();
  }

  function handleResize() {
    fitAddon?.fit();
  }

  onDestroy(() => {
    stopTerminal();
  });
</script>

<svelte:window on:resize={handleResize} on:keydown={handleTerminalKey} />

<div class="terminal-popup">
  <Popup {show} closePopup={handleCloseBtn} showCloseBtn={true}>
    <h3 class="text-lg font-bold">Terminal</h3>
    <div bind:this={termContainer} class="terminal-container"></div>
  </Popup>
</div>

<style>
  .terminal-container {
    width: 100%;
    height: 60vh;
    padding-left: 0.5rem;
  }

  :global(.terminal-popup .popup-content) {
    max-width: 56rem;
    max-height: 90vh;
    padding: 0.5rem !important;
  }
</style>
