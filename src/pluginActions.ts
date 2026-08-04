import { addToast } from "./stores";
import { refreshList } from "./sidebar/FileControlSection.svelte";
import type { PluginAction } from "./types/setting";

export interface PluginActionHandlers {
  onShowResult?: (title: string, body: string, pages?: any[]) => void;
  onDownloadFiles?: (items: any[]) => void;
}

/**
 * PluginAction 목록을 실행하는 단일 디스패처.
 * App(훅) / PluginPanel(수동 실행) / PluginInputPopup(입력 후 실행) 세 곳에서
 * 각자 구현하던 분기를 통합 — refresh_tree가 경로에 따라 동작하지 않던
 * 문제(트리 조회만 하고 스토어 갱신 누락)를 함께 해결한다.
 */
export function dispatchPluginActions(
  actions: PluginAction[] | null | undefined,
  handlers: PluginActionHandlers = {},
): void {
  if (!actions) return;
  for (const action of actions) {
    if (action.type === "toast" && action.content) {
      addToast(
        action.content.message,
        action.content.toast_type === "success" ? "success" : "error",
      );
    } else if (action.type === "refresh_tree") {
      refreshList();
    } else if (action.type === "show_result" && action.content) {
      handlers.onShowResult?.(action.content.title, action.content.body ?? "", action.content.pages);
    } else if (action.type === "download_files" && action.content) {
      handlers.onDownloadFiles?.(action.content.items);
    }
  }
}
