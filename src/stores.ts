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

// ---------- 컨텍스트 ----------
export const GLOBAL_FUNCTIONS = Symbol('globalFunctions');
export interface GlobalFunctions {
  refreshList: () => Promise<void>;
}