import { get, writable } from 'svelte/store';
import { draggingInfo, isEditingFileName } from '../stores';

// Pointer-based drag & drop for the sidebar file tree.
// Tauri's native drag-drop handler (dragDropEnabled, needed by the file
// explorer upload) swallows HTML5 drag events on Windows, so the tree
// implements moving via pointer events instead.

export const dropTargetPath = writable<string | null>(null);

const DRAG_THRESHOLD_PX = 5;
const SCROLL_EDGE_PX = 24;
const SCROLL_SPEED_PX = 8;
/// 드래그 중 닫힌 폴더/섹션 위에 이 시간만큼 머물면 자동으로 펼침
export const HOVER_EXPAND_MS = 600;

type MoveHandler = (src: string, dstDir: string) => Promise<void>;
let moveHandler: MoveHandler | null = null;

export function registerMoveHandler(fn: MoveHandler) {
    moveHandler = fn;
}

let pending: { path: string; x: number; y: number } | null = null;
let active = false;
let lastX = 0;
let lastY = 0;
let ghostEl: HTMLDivElement | null = null;
let ghostName = "";
let rafId: number | null = null;

export function onNodePointerDown(event: PointerEvent, path: string) {
    if (event.button !== 0) return;
    if (get(isEditingFileName)) return;
    event.stopPropagation();
    pending = { path, x: event.clientX, y: event.clientY };
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', cancelDrag);
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('blur', cancelDrag);
}

function onPointerMove(event: PointerEvent) {
    if (!pending) return;
    // pointerup was missed (released outside the window) — abort
    if (event.buttons === 0) {
        cleanup();
        return;
    }
    if (!active) {
        const dx = event.clientX - pending.x;
        const dy = event.clientY - pending.y;
        if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return;
        active = true;
        draggingInfo.set({ path: pending.path });
        document.body.classList.add('tree-dragging');
        showGhost(pending.path.split('/').pop() ?? pending.path);
        rafId = requestAnimationFrame(autoScrollLoop);
    }
    lastX = event.clientX;
    lastY = event.clientY;
    moveGhost();
    setDropTarget(findDropTarget(lastX, lastY));
}

/// 드롭 대상 갱신 + 고스트 라벨에 목적지 표시 ("이름 → 폴더")
function setDropTarget(target: string | null) {
    dropTargetPath.set(target);
    if (!ghostEl) return;
    if (target) {
        const destName = target.split('/').filter(Boolean).pop() ?? target;
        ghostEl.textContent = `${ghostName} → ${destName}`;
        ghostEl.classList.add('on-target');
    } else {
        ghostEl.textContent = ghostName;
        ghostEl.classList.remove('on-target');
    }
}

function findDropTarget(x: number, y: number): string | null {
    const el = document.elementFromPoint(x, y)?.closest('[data-drop-dir]');
    const dir = el?.getAttribute('data-drop-dir');
    if (!dir || !pending) return null;
    // Not onto itself or into its own subtree
    if (dir === pending.path || dir.startsWith(pending.path + '/')) return null;
    return dir;
}

// Scroll the tree list while hovering near its top/bottom edge
function autoScrollLoop() {
    rafId = requestAnimationFrame(autoScrollLoop);
    const container = document.elementFromPoint(lastX, lastY)?.closest('.section-content');
    if (!(container instanceof HTMLElement)) return;
    const rect = container.getBoundingClientRect();
    let scrolled = false;
    if (lastY < rect.top + SCROLL_EDGE_PX && container.scrollTop > 0) {
        container.scrollTop -= SCROLL_SPEED_PX;
        scrolled = true;
    } else if (lastY > rect.bottom - SCROLL_EDGE_PX) {
        container.scrollTop += SCROLL_SPEED_PX;
        scrolled = true;
    }
    if (scrolled) {
        setDropTarget(findDropTarget(lastX, lastY));
    }
}

function showGhost(name: string) {
    ghostName = name;
    ghostEl = document.createElement('div');
    ghostEl.className = 'tree-drag-ghost';
    ghostEl.textContent = name;
    document.body.appendChild(ghostEl);
    moveGhost();
}

function moveGhost() {
    if (ghostEl) {
        ghostEl.style.transform = `translate(${lastX + 14}px, ${lastY + 14}px)`;
    }
}

async function onPointerUp() {
    const wasActive = active;
    const src = pending?.path;
    const dstDir = get(dropTargetPath);
    cleanup();
    if (!wasActive) return;
    // Swallow the click that follows a drag so it doesn't select/open the source
    window.addEventListener('click', suppressClick, { capture: true, once: true });
    setTimeout(() => window.removeEventListener('click', suppressClick, { capture: true }), 0);
    if (src && dstDir && moveHandler) {
        await moveHandler(src, dstDir);
    }
}

function suppressClick(event: MouseEvent) {
    event.stopPropagation();
    event.preventDefault();
}

function onKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') cancelDrag();
}

function cancelDrag() {
    cleanup();
}

function cleanup() {
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);
    window.removeEventListener('pointercancel', cancelDrag);
    window.removeEventListener('keydown', onKeyDown);
    window.removeEventListener('blur', cancelDrag);
    if (rafId !== null) {
        cancelAnimationFrame(rafId);
        rafId = null;
    }
    ghostEl?.remove();
    ghostEl = null;
    document.body.classList.remove('tree-dragging');
    pending = null;
    active = false;
    draggingInfo.set(null);
    dropTargetPath.set(null);
}
