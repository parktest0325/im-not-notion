import { writable } from 'svelte/store';

// ---------- 상태 ----------
export const relativeFilePath = writable<string>("");
export const selectedCursor = writable<string>("");
export const isConnected = writable(false);
export const url = writable<string>("");
export const contentPath = writable<string>("");
export const hiddenPath = writable<string>("");
export const fullFilePath = writable<string>("");
export const draggingInfo = writable<{
  path: string;
} | null>(null);
// Indicates whether any filename is currently being edited.
export const isEditingFileName = writable(false);

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
  setTimeout(() => {
    toasts.update(t => t.filter(item => item.id !== id));
  }, 3000);
}
