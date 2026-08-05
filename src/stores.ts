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
// ---------- 탭 ----------

// 열려있는 파일 탭 목록 (relativeFilePath 형식 경로)
export const openTabs = writable<string[]>([]);

/** 파일/폴더 이름변경·이동 시 탭 경로 갱신 (폴더면 하위 탭도 prefix 치환) */
export function renameOpenTabs(oldPath: string, newPath: string) {
  openTabs.update(tabs => tabs.map(t => {
    if (t === oldPath) return newPath;
    if (t.startsWith(oldPath + "/")) return newPath + t.slice(oldPath.length);
    return t;
  }));
}

/** 삭제된 파일/폴더의 탭 제거 (복원 히스토리에는 넣지 않음 — 파일이 사라졌으므로) */
export function closeTabsUnder(path: string) {
  openTabs.update(tabs => tabs.filter(t => t !== path && !t.startsWith(path + "/")));
}

// 최근 닫은 탭 히스토리 (Mod+Shift+T 복원용, LIFO)
const closedTabHistory: string[] = [];

export function pushClosedTab(path: string) {
  closedTabHistory.push(path);
  if (closedTabHistory.length > 50) closedTabHistory.shift();
}

/** 현재 열려있지 않은 가장 최근 닫은 탭을 꺼냄 */
export function popClosedTab(openNow: string[]): string | undefined {
  while (closedTabHistory.length > 0) {
    const p = closedTabHistory.pop()!;
    if (!openNow.includes(p)) return p;
  }
  return undefined;
}

export function clearClosedTabs() {
  closedTabHistory.length = 0;
}

// 섹션 전체 펼치기/접기 신호: prefix(예: "/blog") 하위의 모든 폴더에 적용.
// exact=true면 해당 경로의 폴더 하나만. 디스패치 직후 null로 초기화된다.
export const treeExpandSignal = writable<{ prefix: string; expand: boolean; seq: number; exact?: boolean } | null>(null);

// 트리 우클릭 컨텍스트 메뉴 (null이면 닫힘). isSection이면 생성 메뉴만 표시.
export const treeContextMenu = writable<{ x: number; y: number; path: string; isDir: boolean; isSection?: boolean } | null>(null);
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
