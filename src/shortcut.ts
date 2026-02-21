import { writable, get } from "svelte/store";

// --- Types ---

interface ParsedShortcut {
  ctrl: boolean;
  meta: boolean;
  shift: boolean;
  alt: boolean;
  key: string; // lowercase
}

interface BuiltinDef {
  shortcuts: string[];
  description: string;
}

// --- Built-in defaults ---

export const BUILTIN_DEFAULTS: Record<string, BuiltinDef> = {
  save:        { shortcuts: ["Ctrl+S", "Meta+S"],  description: "Save file" },
  "exit-edit": { shortcuts: ["Escape"],        description: "Exit edit mode" },
  rename:      { shortcuts: ["F2", "Enter"],   description: "Rename file" },
};

// --- Reserved shortcuts (browser/system defaults — never intercepted) ---

const RESERVED_SHORTCUTS = new Set([
  "Ctrl+f", "Meta+f",           // find (CodeMirror handles)
  "Ctrl+h", "Meta+h",           // replace (CodeMirror handles)
  "Ctrl+c", "Meta+c",           // copy
  "Ctrl+v", "Meta+v",           // paste
  "Ctrl+x", "Meta+x",           // cut
  "Ctrl+a", "Meta+a",           // select all
  "Ctrl+z", "Meta+z",           // undo
  "Ctrl+Shift+z", "Meta+Shift+z", // redo
  "Ctrl+r", "Meta+r",           // reload (dev)
  "Ctrl+Shift+r", "Meta+Shift+r", // hard reload (dev)
  "Ctrl+Shift+i", "Meta+Alt+i", // dev tools
  "f12",                         // dev tools
]);

// --- Internal state ---

const actionHandlers = new Map<string, () => void>();

// shortcutString (normalized) → action_id
export const resolvedMap = writable<Map<string, string>>(new Map());

// Recording mode — when true, global handler is bypassed
export const isRecordingShortcut = writable(false);

// Plugin shortcut definitions — set when plugins load
export interface PluginShortcutDef {
  id: string;
  shortcut?: string;  // optional — plugin.json may not have a default
  description: string;
}
export const pluginShortcutDefs = writable<PluginShortcutDef[]>([]);

// Cached client overrides — updated when config loads
let cachedClientOverrides: Record<string, string[]> = {};

// --- Shortcut string parsing ---

function parseShortcut(str: string): ParsedShortcut {
  const parts = str.split("+");
  const modifiers = parts.slice(0, -1).map((m) => m.toLowerCase());
  const key = parts[parts.length - 1].toLowerCase();

  return {
    ctrl: modifiers.includes("ctrl"),
    meta: modifiers.includes("meta") || modifiers.includes("cmd"),
    shift: modifiers.includes("shift"),
    alt: modifiers.includes("alt"),
    key,
  };
}

function normalizeKey(key: string): string {
  if (key === " ") return "space";
  return key.toLowerCase();
}

function eventToString(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.metaKey) parts.push("Meta");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  parts.push(normalizeKey(e.key));
  return parts.join("+");
}

function shortcutToNormalized(str: string): string {
  const parsed = parseShortcut(str);
  const parts: string[] = [];
  if (parsed.ctrl) parts.push("Ctrl");
  if (parsed.meta) parts.push("Meta");
  if (parsed.shift) parts.push("Shift");
  if (parsed.alt) parts.push("Alt");
  parts.push(parsed.key);
  return parts.join("+");
}

/**
 * Expand "CmdOrCtrl+X" into two entries: "Ctrl+X" and "Meta+X".
 * Other shortcuts pass through unchanged.
 */
function expandCmdOrCtrl(shortcuts: string[]): string[] {
  const result: string[] = [];
  for (const s of shortcuts) {
    if (s.includes("CmdOrCtrl")) {
      result.push(s.replace("CmdOrCtrl", "Ctrl"));
      result.push(s.replace("CmdOrCtrl", "Meta"));
    } else {
      result.push(s);
    }
  }
  return result;
}

// --- Display helpers ---

/**
 * Convert a KeyboardEvent to a display-friendly shortcut string for recording.
 * Returns null if only modifier keys are pressed (no actual key yet).
 */
export function eventToShortcutString(e: KeyboardEvent): string | null {
  // Skip if only modifier keys are pressed
  if (["Control", "Meta", "Shift", "Alt"].includes(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.metaKey) parts.push("Meta");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");

  // Capitalize first letter for display
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  parts.push(key);
  return parts.join("+");
}

/**
 * Get the effective shortcuts for all known actions.
 * Returns a list of { id, description, shortcuts } merging defaults + plugins with overrides.
 */
export function getEffectiveShortcuts(
  clientOverrides: Record<string, string[]>,
): Array<{ id: string; description: string; shortcuts: string[] }> {
  const result: Array<{ id: string; description: string; shortcuts: string[] }> = [];
  const seen = new Set<string>();

  // 1. Built-in defaults
  for (const [id, def] of Object.entries(BUILTIN_DEFAULTS)) {
    result.push({
      id,
      description: def.description,
      shortcuts: clientOverrides[id] ? [...clientOverrides[id]] : [...def.shortcuts],
    });
    seen.add(id);
  }

  // 2. Plugin shortcuts
  const plugins = get(pluginShortcutDefs);
  for (const ps of plugins) {
    if (!seen.has(ps.id)) {
      const defaults = ps.shortcut ? [ps.shortcut] : [];
      result.push({
        id: ps.id,
        description: ps.description,
        shortcuts: clientOverrides[ps.id] ? [...clientOverrides[ps.id]] : defaults,
      });
      seen.add(ps.id);
    }
  }

  // 3. Client overrides for unknown actions (custom entries)
  for (const [id, keys] of Object.entries(clientOverrides)) {
    if (!seen.has(id)) {
      result.push({ id, description: id, shortcuts: [...keys] });
    }
  }

  return result;
}

// --- Public API ---

export function registerAction(id: string, handler: () => void): void {
  actionHandlers.set(id, handler);
}

export function unregisterAction(id: string): void {
  actionHandlers.delete(id);
}

/**
 * Build the resolved shortcut map by merging 3 sources.
 * Either parameter can be omitted to reuse cached values.
 */
export function buildShortcutMap(
  clientOverrides?: Record<string, string[]>,
  pluginShortcuts?: Array<{ id: string; shortcut?: string }>,
): void {
  if (clientOverrides !== undefined) {
    cachedClientOverrides = clientOverrides;
  }
  const overrides = cachedClientOverrides;
  const plugins = pluginShortcuts ?? get(pluginShortcutDefs).map(d => ({ id: d.id, shortcut: d.shortcut }));

  // Step 1: Start with builtin defaults
  const actionToKeys: Record<string, string[]> = {};
  for (const [id, def] of Object.entries(BUILTIN_DEFAULTS)) {
    actionToKeys[id] = [...def.shortcuts];
  }

  // Step 2: Add plugin defaults (only if not already in builtins)
  for (const ps of plugins) {
    if (!(ps.id in actionToKeys) && ps.shortcut) {
      actionToKeys[ps.id] = [ps.shortcut];
    }
  }

  // Step 3: Client overrides replace entirely
  for (const [id, keys] of Object.entries(overrides)) {
    actionToKeys[id] = [...keys];
  }

  // Step 4: Build reverse map (shortcutString → actionId)
  const newMap = new Map<string, string>();
  for (const [actionId, shortcuts] of Object.entries(actionToKeys)) {
    const expanded = expandCmdOrCtrl(shortcuts);
    for (const s of expanded) {
      const normalized = shortcutToNormalized(s);
      newMap.set(normalized, actionId);
    }
  }

  resolvedMap.set(newMap);
}

/**
 * Global keydown handler — call from App.svelte's <svelte:window on:keydown>.
 */
export function handleShortcutEvent(event: KeyboardEvent): void {
  // Skip during shortcut recording mode
  if (get(isRecordingShortcut)) return;

  // Skip if target is an input/textarea and the shortcut is a simple key (no modifiers)
  const target = event.target as HTMLElement;
  const isInput =
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.isContentEditable;
  const hasModifier = event.ctrlKey || event.metaKey || event.altKey;

  const evtStr = eventToString(event);

  // Reserved shortcuts — always pass through to browser/system
  if (RESERVED_SHORTCUTS.has(evtStr)) return;

  const map = get(resolvedMap);
  const actionId = map.get(evtStr);

  if (!actionId) return;

  // For simple keys (no modifier) inside input fields, skip dispatch
  // to allow normal typing. Exception: Escape and F-keys always dispatch.
  const isFunctionKey = event.key.startsWith("F") && event.key.length > 1;
  if (isInput && !hasModifier && event.key !== "Escape" && !isFunctionKey) {
    return;
  }

  const handler = actionHandlers.get(actionId);
  if (handler) {
    event.preventDefault();
    handler();
  }
}
