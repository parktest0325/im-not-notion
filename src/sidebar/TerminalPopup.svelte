<script lang="ts">
  import Popup from "../component/Popup.svelte";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { onDestroy, tick } from "svelte";
  import { addToast } from "../stores";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";

  export let show: boolean;
  export let closeTerminal: () => void;

  let termContainer: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let started = false;

  async function startTerminal() {
    if (started) return;
    await tick();
    if (!termContainer) return;

    terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
      theme: {
        background: "#1e1e1e",
        foreground: "#d4d4d4",
        cursor: "#d4d4d4",
      },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(termContainer);
    fitAddon.fit();

    const { cols, rows } = terminal;

    // Tauri Channel: 서버 출력 수신
    const onEvent = new Channel<string>();
    onEvent.onmessage = (data: string) => {
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

    // 사용자 입력 → 서버로 전송
    terminal.onData((data: string) => {
      invoke("write_pty_cmd", { data }).catch((e: unknown) => {
        console.error("write_pty_cmd error:", e);
      });
    });

    // 터미널 리사이즈 → 서버 PTY 리사이즈
    terminal.onResize(({ cols, rows }: { cols: number; rows: number }) => {
      invoke("resize_pty_cmd", { cols, rows }).catch((e: unknown) => {
        console.error("resize_pty_cmd error:", e);
      });
    });
  }

  function stopTerminal() {
    if (!started) return;
    invoke("stop_pty_cmd").catch((e: unknown) => {
      console.error("stop_pty_cmd error:", e);
    });
    terminal?.dispose();
    terminal = null;
    fitAddon = null;
    started = false;
  }

  // show 변경 감시
  $: if (show) {
    startTerminal();
  } else if (!show && started) {
    stopTerminal();
  }

  // 팝업 크기 변경 시 터미널 리사이즈
  function handleResize() {
    fitAddon?.fit();
  }

  onDestroy(() => {
    stopTerminal();
  });
</script>

<svelte:window on:resize={handleResize} />

<div class="terminal-popup">
  <Popup {show} closePopup={closeTerminal}>
    <div bind:this={termContainer} class="terminal-container"></div>
  </Popup>
</div>

<style>
  .terminal-container {
    width: 100%;
    height: 60vh;
  }

  :global(.terminal-popup .popup-content) {
    max-width: 56rem;
    max-height: 90vh;
    padding: 0.5rem !important;
  }
</style>
