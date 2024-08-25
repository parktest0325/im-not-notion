import { writable } from 'svelte/store';

export const selectedFilePath = writable<string>("");
export const selectedCursor = writable<string>("");
export const isConnected = writable(false);