import type { Terminal } from "@xterm/xterm";

type PtyWriter = (data: string) => Promise<unknown>;

const ALT_SCREEN_ENTER = "\x1b[?1049h";
const ALT_SCREEN_EXIT = "\x1b[?1049l";

export class TerminalInputController {
  private inputBuffer = "";
  private inputFlushQueued = false;
  private inputWriteInFlight = false;
  private isAltScreen = false;
  private disposed = false;

  constructor(
    private readonly terminal: Terminal,
    private readonly writePty: PtyWriter,
  ) {}

  attach(): void {
    this.terminal.attachCustomKeyEventHandler((e: KeyboardEvent) => this.handleCustomKey(e));
    this.terminal.onData((data: string) => this.handleOnData(data));
  }

  dispose(): void {
    this.disposed = true;
    this.inputBuffer = "";
    // Reset custom handler to default pass-through behavior.
    this.terminal.attachCustomKeyEventHandler(() => true);
  }

  handleOutput(data: string): void {
    if (data.includes(ALT_SCREEN_ENTER)) this.isAltScreen = true;
    if (data.includes(ALT_SCREEN_EXIT)) this.isAltScreen = false;
  }

  private handleOnData(data: string): void {
    if (this.disposed) return;
    // In alt-screen mode we rely on custom-key fallback path only.
    if (this.isAltScreen) return;
    this.enqueue(data);
  }

  private handleCustomKey(e: KeyboardEvent): boolean {
    if (this.disposed) return true;
    if (!this.isAltScreen || e.type !== "keydown") return true;

    const fallback = this.getAltScreenFallbackInput(e);
    if (fallback === null) return true;

    this.enqueue(fallback);
    // Block xterm default processing to avoid duplicate sends.
    return false;
  }

  private enqueue(data: string): void {
    this.inputBuffer += data;
    this.queueFlush();
  }

  private queueFlush(): void {
    if (this.inputFlushQueued) return;
    this.inputFlushQueued = true;

    queueMicrotask(() => {
      this.inputFlushQueued = false;
      if (this.disposed || this.inputWriteInFlight || !this.inputBuffer) return;

      const buf = this.inputBuffer;
      this.inputBuffer = "";
      this.inputWriteInFlight = true;

      this.writePty(buf)
        .catch((e: unknown) => {
          console.error("write_pty_cmd error:", e);
        })
        .finally(() => {
          this.inputWriteInFlight = false;
          if (!this.disposed && this.inputBuffer) this.queueFlush();
        });
    });
  }

  private getAltScreenFallbackInput(e: KeyboardEvent): string | null {
    // Ctrl+letter → control character (Ctrl+C=0x03, Ctrl+R=0x12 등)
    if (e.ctrlKey && !e.metaKey && !e.altKey && e.key.length === 1) {
      const code = e.key.toUpperCase().charCodeAt(0) - 64;
      if (code >= 0 && code < 32) return String.fromCharCode(code);
      return null;
    }
    if (e.metaKey || e.altKey) return null;

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
}

