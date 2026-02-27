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
  let fontSize = 14;
  const ALT_SCREEN_ENTER = "\x1b[?1049h";
  const ALT_SCREEN_EXIT = "\x1b[?1049l";

  function getAltScreenFallbackInput(e: KeyboardEvent): string | null {
    if (e.ctrlKey || e.metaKey || e.altKey) return null;

    if (e.key === "Enter") return "\r";
    if (e.key === "Backspace") return "\x7f";
    if (e.key === "Tab") return "\t";
    if (e.key === "Escape") return "\x1b";

    if (e.key.length === 1) return e.key;

    if (e.key === "Process" || e.key === "Unidentified") {
      if (e.code.startsWith("Key")) {
        const ch = e.code.slice(3, 4);
        return e.shiftKey ? ch : ch.toLowerCase();
      }
      if (e.code.startsWith("Digit")) {
        return e.code.slice(5, 6);
      }
    }

    switch (e.key) {
      case "ArrowUp": return "\x1b[A";
      case "ArrowDown": return "\x1b[B";
      case "ArrowRight": return "\x1b[C";
      case "ArrowLeft": return "\x1b[D";
      case "Home": return "\x1b[H";
      case "End": return "\x1b[F";
      case "Delete": return "\x1b[3~";
      case "Insert": return "\x1b[2~";
      case "PageUp": return "\x1b[5~";
      case "PageDown": return "\x1b[6~";
      default: return null;
    }
  }

  function changeFontSize(delta: number) {
    fontSize = Math.max(8, Math.min(32, fontSize + delta));
    if (terminal) {
      terminal.options.fontSize = fontSize;
      fitAddon?.fit();
    }
  }

  function handleTerminalKey(e: KeyboardEvent) {
    if (!show || !started) return;
    if (e.ctrlKey && (e.key === '=' || e.key === '+')) {
      e.preventDefault();
      changeFontSize(1);
    } else if (e.ctrlKey && e.key === '-') {
      e.preventDefault();
      changeFontSize(-1);
    } else if (e.ctrlKey && e.key === '0') {
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
        background: cs.getPropertyValue('--terminal-bg').trim() || "#1e1e1e",
        foreground: cs.getPropertyValue('--terminal-fg').trim() || "#d4d4d4",
        cursor: cs.getPropertyValue('--terminal-cursor').trim() || "#d4d4d4",
      },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(termContainer);
    fitAddon.fit();
    terminal.focus();

    const { cols, rows } = terminal;
    let isAltScreen = false;

    terminal.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (!isAltScreen || e.type !== "keydown") return true;

      const fallback = getAltScreenFallbackInput(e);
      if (fallback === null) return true;

      invoke("write_pty_cmd", { data: fallback }).catch((err: unknown) => {
        console.error("alt-screen write_pty_cmd error:", err);
      });
      return false;
    });

    // Tauri Channel: 서버 출력 수신
    const onEvent = new Channel<string>();
    onEvent.onmessage = (data: string) => {
      if (data === "\x00__PTY_CLOSED__") {
        // 셸 종료 (exit) → 터미널 팝업 닫기
        stopTerminal();
        closeTerminal();
        return;
      }
      if (data.includes(ALT_SCREEN_ENTER)) isAltScreen = true;
      if (data.includes(ALT_SCREEN_EXIT)) isAltScreen = false;
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

    // 사용자 입력은 타이머 대신 마이크로태스크 단위로 flush.
    // 릴리즈에서 타이머 지연으로 입력 전송이 밀리는 케이스를 피한다.
    let inputBuffer = "";
    let inputFlushQueued = false;
    let inputWriteInFlight = false;

    const queueInputFlush = () => {
      if (inputFlushQueued) return;
      inputFlushQueued = true;
      queueMicrotask(() => {
        inputFlushQueued = false;
        if (inputWriteInFlight || !inputBuffer) return;

        const buf = inputBuffer;
        inputBuffer = "";
        inputWriteInFlight = true;

        invoke("write_pty_cmd", { data: buf }).catch((e: unknown) => {
          console.error("write_pty_cmd error:", e);
        }).finally(() => {
          inputWriteInFlight = false;
          if (inputBuffer) queueInputFlush();
        });
      });
    };

    terminal.onData((data: string) => {
      if (isAltScreen) return;
      inputBuffer += data;
      queueInputFlush();
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

<svelte:window on:resize={handleResize} on:keydown={handleTerminalKey} />

<div class="terminal-popup">
  <Popup {show} closePopup={closeTerminal} showCloseBtn={false}>
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
