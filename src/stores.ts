import { writable } from 'svelte/store';

// ---------- 상태 ----------
export const relativeFilePath = writable<string>("");
export const selectedCursor = writable<string>("");
export const isConnected = writable(false);
export const activeServerName = writable<string>("");
export const url = writable<string>("");
export const contentPaths = writable<string[]>([]);
export const hiddenPath = writable<string>("");
export const fullFilePath = writable<string>("");
export const draggingInfo = writable<{
  path: string;
} | null>(null);
// Indicates whether any filename is currently being edited.
export const isEditingFileName = writable(false);
// Indicates whether content editor is active.
export const isEditingContent = writable(false);
// Set to a file path to trigger rename mode on the matching TreeNode.
export const renamingPath = writable<string>("");
// Set to a line number to jump to after loading a file (0 = ignore).
export const gotoLine = writable<number>(0);
// 마지막 저장 시각 (상태바 표시용)
export const lastSavedAt = writable<Date | null>(null);
// 섹션 전체 펼치기/접기 신호: prefix(예: "/blog") 하위의 모든 폴더에 적용.
// 디스패치 직후 null로 초기화된다 (이후 마운트되는 노드에 잔존 적용 방지).
export const treeExpandSignal = writable<{ prefix: string; expand: boolean; seq: number } | null>(null);
// Set to trigger a plugin via shortcut: { pluginName, triggerLabel, inputFields }
export const triggerPluginShortcut = writable<{
  pluginName: string;
  triggerLabel: string;
} | null>(null);

// ---------- 토스트 ----------

export interface ToastItem {
  id: number;
  message: string;
  type: "error" | "success" | "info";
}

export const toasts = writable<ToastItem[]>([]);

let toastId = 0;
export function addToast(message: string, type: ToastItem["type"] = "error") {
  const id = ++toastId;
  toasts.update(t => [...t, { id, message, type }]);
}
