import { writable } from 'svelte/store';

export const selectedFilePath = writable<string>("");
export const selectedCursor = writable<string>("");
export const isConnected = writable(false);
export const url = writable<string>("");
export const contentPath = writable<string>("");
export const draggingPath = writable<string | null>(null);
// Indicates whether any filename is currently being edited.
export const isEditingFileName = writable(false);