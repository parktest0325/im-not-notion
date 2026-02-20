import { writable } from "svelte/store";

export type Theme = "system" | "light" | "dark";

const STORAGE_KEY = "inn-theme";

function getStoredTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "light" || stored === "dark" || stored === "system") return stored;
  return "system";
}

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function applyTheme(theme: Theme) {
  const isDark =
    theme === "dark" || (theme === "system" && systemPrefersDark());
  document.documentElement.classList.toggle("dark", isDark);
}

export const currentTheme = writable<Theme>(getStoredTheme());

// Apply on init
applyTheme(getStoredTheme());

// React to store changes
currentTheme.subscribe((theme) => {
  localStorage.setItem(STORAGE_KEY, theme);
  applyTheme(theme);
});

// Listen for OS theme changes (only matters when theme === "system")
window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", () => {
    const theme = getStoredTheme();
    if (theme === "system") applyTheme("system");
  });

export function cycleTheme() {
  currentTheme.update((t) => {
    if (t === "system") return "light";
    if (t === "light") return "dark";
    return "system";
  });
}
