<script lang="ts">
  import { isConnected, relativeFilePath, selectedCursor, isEditingContent, addToast, gotoLine, lastSavedAt, url } from "../stores";
  import { invoke } from "@tauri-apps/api/core";
  import { v4 as uuidv4 } from "uuid";
  import { tick, onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import SavePopup from "./SavePopup.svelte";
  import LogoSVG from "../resource/LogoSVG.svelte";
  import { registerAction, unregisterAction } from "../shortcut";

  // CodeMirror
  import { EditorView, keymap, drawSelection, Decoration, ViewPlugin, WidgetType } from "@codemirror/view";
  import type { DecorationSet, ViewUpdate } from "@codemirror/view";
  import { EditorState, EditorSelection, Compartment, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap, indentMore, indentLess } from "@codemirror/commands";
  import { indentUnit } from "@codemirror/language";
  import { search, searchKeymap, searchPanelOpen } from "@codemirror/search";
  import { markdown } from "@codemirror/lang-markdown";
  import { languages } from "@codemirror/language-data";
  import { syntaxHighlighting, HighlightStyle, syntaxTree, LanguageDescription, StreamLanguage } from "@codemirror/language";
  import type { Language } from "@codemirror/language";
  import { tags, highlightCode } from "@lezer/highlight";
  import { StyleModule } from "style-mod";

  let fileContent: string = "";
  let editable: boolean = false;
  let showDialog: boolean = false;
  let contentDiv: HTMLDivElement;
  let editorContainer: HTMLDivElement;
  let targetLine: number = 0;
  let targetYFraction: number = 0;

  let isContentChanged: boolean = false;
  // 저장 await 중의 타이핑을 감지하기 위한 변경 세대 카운터
  let changeGen: number = 0;
  // 파일 전환 경합 방지: 마지막 로드 요청만 유효
  let loadSeq: number = 0;
  const autoSaveInterval = 1000 * 5;
  let autoSaveTimer: number | null = null;

  // Search highlight
  let highlightLine: number = 0;
  let highlightEl: HTMLDivElement | null = null;

  // Unsaved changes dialog
  let currentFilePath: string = "";
  let showUnsavedDialog: boolean = false;
  let unsavedAction: "exit" | "switch" | null = null;
  let unsavedSwitchPath: string | null = null;
  let unsavedSwitchCursor: string | null = null;

  // CodeMirror
  let view: EditorView | null = null;
  const editableCompartment = new Compartment();

  // 앱 CSS 변수 기반 테마
  const innTheme = EditorView.theme({
    "&": {
      backgroundColor: "var(--content-bg-color)",
      color: "var(--reverse-primary-color)",
      height: "100%",
      fontSize: "inherit",
      border: "1px solid var(--border-color)",
      borderRadius: "8px",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-scroller": {
      fontFamily: "inherit",
      lineHeight: "inherit",
      overflow: "auto",
    },
    ".cm-content": {
      padding: "1rem",
      whiteSpace: "pre-wrap",
      wordBreak: "break-all",
      // drawSelection이 .cm-cursor를 그리므로 네이티브 캐럿은 숨김
      caretColor: "transparent",
    },
    ".cm-line": {
      padding: "0",
    },
    ".cm-gutters": {
      display: "none",
    },
    ".cm-cursor": {
      borderLeftColor: "var(--accent-color)",
      borderLeftWidth: "3px",
    },
    ".cm-selectionBackground": {
      backgroundColor: "rgba(45, 212, 191, 0.22) !important",
    },
    "&.cm-focused .cm-selectionBackground": {
      backgroundColor: "rgba(45, 212, 191, 0.3) !important",
    },
    // 선택 레이어를 콘텐츠 위로 — 기본(-2)에서는 코드블록 줄의 불투명
    // 배경에 가려 선택 영역이 보이지 않는다 (색이 반투명이라 텍스트 위에 얹혀도 자연스러움)
    ".cm-selectionLayer": {
      zIndex: "1 !important",
      pointerEvents: "none",
    },
    // 검색 하이라이트
    ".cm-searchMatch": {
      backgroundColor: "var(--search-match-bg)",
      borderRadius: "2px",
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
      backgroundColor: "var(--search-match-current-bg)",
    },
    // 검색 패널
    ".cm-panel.cm-search": {
      backgroundColor: "var(--search-bar-bg)",
      borderBottom: "1px solid var(--search-bar-border)",
      padding: "4px 8px",
    },
    ".cm-panel.cm-search input": {
      backgroundColor: "var(--input-bg-color)",
      color: "var(--reverse-primary-color)",
      border: "1px solid var(--border-color)",
      borderRadius: "3px",
      padding: "2px 6px",
      fontSize: "0.8rem",
      outline: "none",
    },
    ".cm-panel.cm-search input:focus": {
      borderColor: "var(--highlight-color)",
    },
    ".cm-panel.cm-search button": {
      backgroundColor: "transparent",
      color: "var(--reverse-primary-color)",
      border: "1px solid var(--border-color)",
      borderRadius: "3px",
      padding: "2px 8px",
      fontSize: "0.75rem",
      cursor: "pointer",
    },
    ".cm-panel.cm-search button:hover": {
      backgroundColor: "var(--button-hover-bg-color)",
    },
    ".cm-panel.cm-search label": {
      color: "var(--reverse-secondary-color)",
      fontSize: "0.75rem",
    },
    ".cm-panel.cm-search .cm-button": {
      backgroundImage: "none",
    },
    // 인라인 이미지 위젯
    ".cm-inline-img-wrap": {
      position: "relative",
      display: "block",
      width: "fit-content",
      maxWidth: "min(100%, 720px)",
      margin: "0.5rem 0",
    },
    ".cm-inline-img": {
      display: "block",
      maxWidth: "100%",
      maxHeight: "60vh",
      borderRadius: "8px",
      border: "1px solid var(--border-color)",
    },
    ".cm-inline-img-broken": {
      opacity: "0.55",
    },
    // </> 로 열린 이미지 마크다운 코드 (숨김 파일과 같은 주황 계열)
    ".cm-img-code-text": {
      color: "var(--reverse-third-color)",
    },
    // ``` 코드블록 줄 배경 (배경과 구분되도록 테두리 포함)
    ".cm-codeblock-line": {
      backgroundColor: "var(--code-block-bg)",
      borderLeft: "1px solid var(--code-block-border)",
      borderRight: "1px solid var(--code-block-border)",
      paddingLeft: "0.75rem",
      paddingRight: "0.75rem",
    },
    ".cm-codeblock-first": {
      borderTop: "1px solid var(--code-block-border)",
      borderTopLeftRadius: "8px",
      borderTopRightRadius: "8px",
    },
    ".cm-codeblock-last": {
      borderBottom: "1px solid var(--code-block-border)",
      borderBottomLeftRadius: "8px",
      borderBottomRightRadius: "8px",
    },
    // 호버 시 나타나는 코드 편집 버튼
    ".cm-img-code-btn": {
      position: "absolute",
      top: "8px",
      right: "8px",
      padding: "2px 8px",
      fontSize: "11px",
      fontFamily: "var(--font-mono)",
      backgroundColor: "var(--popup-bg-color)",
      color: "var(--reverse-secondary-color)",
      border: "1px solid var(--border-color)",
      borderRadius: "6px",
      cursor: "pointer",
      opacity: "0",
      transition: "opacity 0.15s",
      boxShadow: "var(--shadow-toast)",
    },
    ".cm-inline-img-wrap:hover .cm-img-code-btn": {
      opacity: "0.95",
    },
    ".cm-img-code-btn:hover": {
      backgroundColor: "var(--button-hover-bg-color)",
    },
  });

  /** 선택 영역을 마커로 감싸거나(볼드/이탤릭/코드/밑줄), 이미 감싸져 있으면 제거 */
  function toggleWrap(v: EditorView, open: string, close: string = open): boolean {
    const openLen = open.length;
    const closeLen = close.length;
    v.dispatch(
      v.state.changeByRange((range) => {
        const { from, to } = range;
        const selText = v.state.sliceDoc(from, to);
        // 선택 안에 마커 포함: "**text**" 전체를 선택한 경우 → 마커 제거
        if (selText.length >= openLen + closeLen && selText.startsWith(open) && selText.endsWith(close)) {
          return {
            changes: { from, to, insert: selText.slice(openLen, selText.length - closeLen) },
            range: EditorSelection.range(from, to - openLen - closeLen),
          };
        }
        // 선택 밖에 마커가 감싸고 있는 경우: **|text|** → 마커 제거
        const before = v.state.sliceDoc(Math.max(0, from - openLen), from);
        const after = v.state.sliceDoc(to, Math.min(v.state.doc.length, to + closeLen));
        if (before === open && after === close) {
          return {
            changes: [
              { from: from - openLen, to: from },
              { from: to, to: to + closeLen },
            ],
            range: EditorSelection.range(from - openLen, to - openLen),
          };
        }
        // 감싸기
        return {
          changes: [
            { from, insert: open },
            { from: to, insert: close },
          ],
          range: EditorSelection.range(from + openLen, to + openLen),
        };
      }),
    );
    return true;
  }

  function createEditorView() {
    if (view) {
      view.destroy();
      view = null;
    }
    if (!editorContainer) return;

    const state = EditorState.create({
      doc: fileContent,
      extensions: [
        editableCompartment.of(EditorView.editable.of(editable)),
        innTheme,
        history(),
        // codeLanguages: ``` 코드블록 내부를 언어별로 하이라이팅 (별칭/tree 포함)
        markdown({
          codeLanguages: (info) => {
            const r = resolveFenceLanguage(info);
            return r?.lang ?? r?.desc ?? null;
          },
        }),
        syntaxHighlighting(innHighlight),
        codeBlockPlugin,
        search(),
        // 커서/선택 영역을 CM이 직접 그리도록 (네이티브 캐럿은 1px 고정이라
        // .cm-cursor 두께/색 스타일이 적용되지 않는다)
        drawSelection(),
        // 이미지 문법을 실제 이미지로 렌더링 (</> 버튼으로 코드 편집)
        revealedImageRange,
        inlineImagePlugin,
        // Tab 들여쓰기 단위: 스페이스 2칸
        indentUnit.of("  "),
        keymap.of([
          // Ctrl+S / Cmd+S → 저장
          { key: "Mod-s", run: () => { if (editable) showDialog = true; return true; } },
          // 마크다운 서식 토글
          { key: "Mod-b", run: (v) => toggleWrap(v, "**") },
          { key: "Mod-i", run: (v) => toggleWrap(v, "*") },
          { key: "Mod-m", run: (v) => toggleWrap(v, "`") },
          // 밑줄은 md 표준에 없어 HTML 태그 사용
          { key: "Mod-u", run: (v) => toggleWrap(v, "<u>", "</u>") },
          // Tab/Shift+Tab: 줄 맨 앞 스페이스 2칸 추가/제거 (여러 줄 지원)
          { key: "Tab", run: indentMore, shift: indentLess },
          // Escape → exit edit (검색 패널이 열려있으면 searchKeymap이 먼저 닫도록 양보)
          { key: "Escape", run: (v) => {
            if (searchPanelOpen(v.state)) return false;
            if (editable) { tryExitEdit(); return true; }
            return false;
          }},
          ...searchKeymap,
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        // 문서 변경 감지
        EditorView.updateListener.of(update => {
          if (update.docChanged) {
            fileContent = update.state.doc.toString();
            isContentChanged = true;
            changeGen++;
          }
        }),
        // 이미지 붙여넣기
        EditorView.domEventHandlers({
          paste: handlePaste,
        }),
        // 줄바꿈
        EditorView.lineWrapping,
      ],
    });

    view = new EditorView({
      state,
      parent: editorContainer,
    });
  }

  // --- Edit mode transitions ---
  // NOTE: view, fileContent must NOT appear directly
  //       in `$:` blocks — Svelte tracks them as reactive dependencies
  //       and would cause infinite re-runs when createEditorView sets `view`.

  function handlePreviewDblClick(e: MouseEvent) {
    if ($relativeFilePath != "") {
      const target = (e.target as HTMLElement).closest('[data-line]');
      targetLine = target ? parseInt(target.getAttribute('data-line')!) : 0;
      // 클릭한 줄의 뷰포트 내 상대 위치 (0=상단, 1=하단)
      if (target) {
        const containerRect = contentDiv.getBoundingClientRect();
        const lineRect = target.getBoundingClientRect();
        targetYFraction = (lineRect.top - containerRect.top) / containerRect.height;
      } else {
        targetYFraction = 0;
      }
      highlightLine = 0;
      editable = true;
    }
  }

  function enterEditMode() {
    isEditingContent.set(true);
    tick().then(() => {
      createEditorView();
      tick().then(() => {
        if (view) {
          view.focus();
          requestAnimationFrame(() => {
            if (!view) return;
            const lineNum = Math.min(targetLine + 1, view.state.doc.lines);
            const line = view.state.doc.line(lineNum);
            const vpHeight = view.scrollDOM.clientHeight;
            // EditorView.scrollIntoView 는 layout이 settle된 뒤 처리되므로
            // 직접 scrollTop을 쓰는 것보다 측정 정확도가 높다.
            // y:"start" + yMargin = 프리뷰에서 클릭한 줄이 뷰포트의 어느 높이에
            // 있었는지를 그대로 재현 (yMargin은 viewport top으로부터의 여백).
            view.dispatch({
              selection: { anchor: line.from },
              effects: EditorView.scrollIntoView(line.from, {
                y: "start",
                yMargin: Math.max(0, Math.min(1, targetYFraction)) * vpHeight,
              }),
            });
          });
        }
      });
      startAutoSave();
    });
  }

  function exitEditMode() {
    isEditingContent.set(false);
    if (view) {
      fileContent = view.state.doc.toString();
      // 커서가 있는 줄 번호와 뷰포트 내 상대 위치 저장
      const cursor = view.state.selection.main.head;
      targetLine = view.state.doc.lineAt(cursor).number - 1;
      const lineBlock = view.lineBlockAt(cursor);
      const vpHeight = view.scrollDOM.clientHeight;
      targetYFraction = vpHeight > 0 ? (lineBlock.top - view.scrollDOM.scrollTop) / vpHeight : 0;
      view.destroy();
      view = null;
    }
    tick().then(() => {
      if (contentDiv) {
        // 프리뷰에서 해당 줄을 같은 뷰포트 비율 위치에 놓기
        const lineEl = contentDiv.querySelector(`[data-line="${targetLine}"]`);
        if (lineEl) {
          const containerRect = contentDiv.getBoundingClientRect();
          const lineTop = (lineEl as HTMLElement).offsetTop - contentDiv.offsetTop;
          contentDiv.scrollTop = lineTop - targetYFraction * containerRect.height;
        }
      }
      stopAutoSave();
    });
  }

  // --- Unsaved changes ---

  function tryExitEdit() {
    if (isContentChanged) {
      stopAutoSave();
      unsavedAction = "exit";
      showUnsavedDialog = true;
    } else {
      editable = false;
    }
  }

  function handleFilePathChange(newPath: string) {
    if (!newPath || newPath === currentFilePath) return;

    if (editable && isContentChanged) {
      stopAutoSave();
      unsavedAction = "switch";
      unsavedSwitchPath = newPath;
      unsavedSwitchCursor = get(selectedCursor);
      // 즉시 되돌려서 TopBar/사이드바가 점프하지 않도록
      relativeFilePath.set(currentFilePath);
      selectedCursor.set(currentFilePath);
      showUnsavedDialog = true;
    } else {
      switchToFile(newPath);
    }
  }

  function switchToFile(path: string, cursor?: string) {
    currentFilePath = path;
    relativeFilePath.set(path);
    if (cursor) selectedCursor.set(cursor);
    highlightLine = 0;
    getFileContent(path);
    contentDiv?.scrollTo(0, 0);
    editable = false;
  }

  async function handleUnsavedSave() {
    // 항상 원래 파일(currentFilePath)에 저장
    await saveContent(true, currentFilePath);

    // 저장 완전 실패 시 (SSH 끊김 등) 편집 모드 유지
    if (isContentChanged) {
      showUnsavedDialog = false;
      unsavedAction = null;
      unsavedSwitchPath = null;
      unsavedSwitchCursor = null;
      startAutoSave();
      return;
    }

    showUnsavedDialog = false;
    addToast("File saved.", "success");

    if (unsavedAction === "exit") {
      await getFileContent(currentFilePath);
      editable = false;
    } else if (unsavedAction === "switch" && unsavedSwitchPath) {
      switchToFile(unsavedSwitchPath, unsavedSwitchCursor ?? undefined);
    }
    unsavedAction = null;
    unsavedSwitchPath = null;
    unsavedSwitchCursor = null;
  }

  async function handleUnsavedDiscard() {
    showUnsavedDialog = false;
    isContentChanged = false;

    if (unsavedAction === "exit") {
      await getFileContent(currentFilePath);
      editable = false;
    } else if (unsavedAction === "switch" && unsavedSwitchPath) {
      switchToFile(unsavedSwitchPath, unsavedSwitchCursor ?? undefined);
    }
    unsavedAction = null;
    unsavedSwitchPath = null;
    unsavedSwitchCursor = null;
  }

  function handleUnsavedCancel() {
    showUnsavedDialog = false;
    // stores는 handleFilePathChange에서 이미 되돌림
    unsavedAction = null;
    unsavedSwitchPath = null;
    unsavedSwitchCursor = null;
    startAutoSave();
  }

  // --- Reactive ---

  $: if ($relativeFilePath) {
    handleFilePathChange($relativeFilePath);
  } else if (currentFilePath) {
    // 열려있던 파일이 삭제된 경우 등 — 에디터를 정리하고 자동저장을 멈춘다
    // (방치하면 자동저장이 삭제된 파일을 서버에 되살린다)
    handleFileCleared();
  }

  function handleFileCleared() {
    stopAutoSave();
    loadSeq++; // 진행 중인 로드 응답 무효화
    currentFilePath = "";
    fileContent = "";
    isContentChanged = false;
    editable = false;
    showUnsavedDialog = false;
    unsavedAction = null;
    unsavedSwitchPath = null;
    unsavedSwitchCursor = null;
    syncEditorContent();
  }

  // 같은 파일 내에서 검색 결과 라인 클릭 시 (relativeFilePath 변경 없이 gotoLine만 변경)
  $: if ($gotoLine > 0 && currentFilePath) {
    highlightLine = $gotoLine;
    gotoLine.set(0);
    scrollToHighlight();
  }

  $: if (editable) {
    enterEditMode();
  } else {
    exitEditMode();
  }

  // fileContent가 외부에서 바뀌었을 때 (getFileContent 호출 등) 에디터에 반영
  function syncEditorContent() {
    if (view && view.state.doc.toString() !== fileContent) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: fileContent },
      });
    }
  }

  // --- 프리뷰 이미지 렌더링 (옵시디언처럼 ![..](..) 를 실제 이미지로) ---

  type LineSegment =
    | { type: "text"; text: string }
    | { type: "img"; alt: string; src: string };

  const IMAGE_MD_RE = /!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;

  function parseLine(line: string): LineSegment[] {
    const segments: LineSegment[] = [];
    let lastIndex = 0;
    for (const m of line.matchAll(IMAGE_MD_RE)) {
      if (m.index! > lastIndex) {
        segments.push({ type: "text", text: line.slice(lastIndex, m.index) });
      }
      segments.push({ type: "img", alt: m[1], src: m[2] });
      lastIndex = m.index! + m[0].length;
    }
    if (lastIndex < line.length) {
      segments.push({ type: "text", text: line.slice(lastIndex) });
    }
    return segments;
  }

  /** 프리뷰용: ``` 펜스 기준으로 줄들을 텍스트/코드블록 그룹으로 묶음 */
  type PreviewCodeBlock = { kind: "code"; lines: { text: string; i: number }[]; closed: boolean };
  type PreviewTextBlock = { kind: "text"; lines: { segments: LineSegment[]; i: number; text: string }[] };
  type PreviewBlock = PreviewCodeBlock | PreviewTextBlock;

  function toPreviewBlocks(content: string): PreviewBlock[] {
    const blocks: PreviewBlock[] = [];
    let codeBlock: PreviewCodeBlock | null = null;
    let textBlock: PreviewTextBlock | null = null;
    content.split("\n").forEach((text, i) => {
      const fence = /^\s*(```|~~~)/.test(text);
      if (codeBlock) {
        codeBlock.lines.push({ text, i });
        if (fence) {
          codeBlock.closed = true;
          codeBlock = null; // 닫는 펜스
        }
      } else if (fence) {
        codeBlock = { kind: "code", lines: [{ text, i }], closed: false };
        blocks.push(codeBlock);
        textBlock = null;
      } else {
        if (!textBlock) {
          textBlock = { kind: "text", lines: [] };
          blocks.push(textBlock);
        }
        textBlock.lines.push({ segments: parseLine(text), i, text });
      }
    });
    return blocks;
  }

  // 코드블록 줄바꿈 모드: 기본은 횡스크롤(줄바꿈 없음), 버튼으로 wrap 전환
  let codeWrap = false;

  // --- 프리뷰 마크다운 하이라이팅 (에디트 모드와 동일한 모습) ---
  // 에디터가 쓰는 innHighlight를 마크다운 파서로 정적 적용한다 —
  // 제목/굵게/기울임 등이 에디트 모드와 똑같은 색으로 보인다.

  const previewMdParser = markdown().language.parser;
  const mdHlCache = new Map<string, { text: string; cls: string | null }[][]>();

  function markdownTokens(text: string): { text: string; cls: string | null }[][] {
    const cached = mdHlCache.get(text);
    if (cached) return cached;
    if (mdHlCache.size > 300) mdHlCache.clear(); // 파일 전환 누적 방지
    const lines: { text: string; cls: string | null }[][] = [[]];
    try {
      const tree = previewMdParser.parse(text);
      highlightCode(
        text,
        tree,
        innHighlight,
        (t, cls) => lines[lines.length - 1].push({ text: t, cls: cls || null }),
        () => lines.push([]),
      );
    } catch {
      return text.split("\n").map((t) => [{ text: t, cls: null }]);
    }
    mdHlCache.set(text, lines);
    return lines;
  }

  // --- 프리뷰 코드블록 신택스 하이라이팅 ---
  // 에디터와 같은 HighlightStyle을 정적으로 적용한다. 언어 파서는 비동기
  // 로드이므로 캐시 + 버전 카운터로 로드 완료 시 재렌더한다.

  type HlToken = { text: string; cls: string | null };
  const hlCache = new Map<string, HlToken[][]>();
  const hlPending = new Set<string>();
  let hlVersion = 0; // 하이라이트 로드 완료 시 재렌더 트리거

  function fenceLang(fenceLine: string): string {
    const m = /^\s*(?:```|~~~)\s*([\w+#-]*)/.exec(fenceLine);
    return m?.[1] ?? "";
  }

  function codeInnerLines(block: PreviewCodeBlock): { text: string; i: number }[] {
    return block.closed ? block.lines.slice(1, -1) : block.lines.slice(1);
  }

  function highlightTokens(block: PreviewCodeBlock, _v: number): HlToken[][] {
    const inner = codeInnerLines(block);
    const lang = fenceLang(block.lines[0].text);
    const code = inner.map((l) => l.text).join("\n");
    const key = `${lang} ${code}`;
    const cached = hlCache.get(key);
    if (cached) return cached;
    void scheduleHighlight(key, lang, code);
    return inner.map((l) => [{ text: l.text, cls: null }]);
  }

  async function scheduleHighlight(key: string, lang: string, code: string) {
    if (!code || hlCache.has(key) || hlPending.has(key)) return;
    const resolved = lang ? resolveFenceLanguage(lang) : null;
    if (!resolved) {
      // 언어 미지정/미지원: 평문 캐시 (재시도 방지)
      hlCache.set(key, code.split("\n").map((t) => [{ text: t, cls: null }]));
      return;
    }
    hlPending.add(key);
    try {
      const language = resolved.lang ?? (await resolved.desc!.load()).language;
      const tree = language.parser.parse(code);
      const lines: HlToken[][] = [[]];
      highlightCode(
        code,
        tree,
        innHighlight,
        (text, classes) => lines[lines.length - 1].push({ text, cls: classes || null }),
        () => lines.push([]),
      );
      hlCache.set(key, lines);
    } catch {
      hlCache.set(key, code.split("\n").map((t) => [{ text: t, cls: null }]));
    } finally {
      hlPending.delete(key);
      hlVersion += 1;
    }
  }

  /** 상대 경로 이미지는 설정된 블로그 URL 기준으로 해석 */
  function resolveImageSrc(src: string, baseUrl: string): string {
    if (/^(https?:)?\/\//.test(src) || src.startsWith("data:")) return src;
    try {
      return new URL(src, baseUrl).toString();
    } catch {
      return src;
    }
  }

  function onPreviewImgError(e: Event) {
    // 로드 실패(서버 미기동, 아직 동기화 전 등) 시 마크다운 원문으로 폴백
    const img = e.currentTarget as HTMLImageElement;
    const fallback = document.createElement("span");
    fallback.className = "preview-img-broken";
    fallback.textContent = `![${img.alt}](${img.dataset.rawSrc ?? img.src})`;
    img.replaceWith(fallback);
  }

  // --- 신택스 하이라이팅 (마크다운 + 코드블록 내부 언어) ---

  const innHighlight = HighlightStyle.define([
    { tag: tags.heading, color: "var(--accent-strong)", fontWeight: "700" },
    { tag: tags.strong, fontWeight: "700" },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strikethrough, textDecoration: "line-through" },
    { tag: [tags.link, tags.url], color: "var(--accent-color)" },
    { tag: tags.quote, color: "var(--code-comment)" },
    { tag: tags.keyword, color: "var(--code-keyword)" },
    { tag: [tags.string, tags.special(tags.string)], color: "var(--code-string)" },
    { tag: [tags.comment, tags.meta], color: "var(--code-comment)", fontStyle: "italic" },
    { tag: [tags.number, tags.bool, tags.atom], color: "var(--code-number)" },
    { tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.className], color: "var(--code-fn)" },
    { tag: [tags.typeName, tags.standard(tags.tagName), tags.tagName], color: "var(--code-type)" },
    { tag: tags.propertyName, color: "var(--code-prop)" },
    { tag: tags.operator, color: "var(--code-operator)" },
    { tag: tags.punctuation, color: "var(--code-comment)" },
    // ```tree 박스 문자 전용 (macroName 태그를 커넥터 색으로 전용)
    { tag: tags.macroName, color: "var(--accent-color)" },
  ]);

  // ```tree 전용 미니 하이라이터: 박스 문자(├ └ │ ─ …)는 액센트 teal,
  // `xhci/` 처럼 /로 끝나는 디렉토리 이름은 타입 색으로 표시
  const treeLanguage = StreamLanguage.define({
    token(stream) {
      if (stream.match(/^[│├└┌┬┴┼─┐┤┘┃┣┗┏━]+/)) return "macroName";
      if (stream.match(/^\S+\//)) return "typeName";
      stream.next();
      return null;
    },
  });

  // ```hexdump 전용: 줄 시작 오프셋은 숫자 색, ASCII 컬럼(|..| 또는 맨 뒤
  // 문자열)은 문자열 색, 컬럼 헤더/반복 표시(*)는 주석 색, 헥사 바이트는 기본 색
  const hexdumpLanguage = StreamLanguage.define({
    token(stream) {
      // 컬럼 인덱스 헤더: "0  1  2  3 ... F  0123456789ABCDEF"
      if (stream.sol() && stream.match(/^\s*0\s+1\s+2\s+3\b.*$/)) return "comment";
      if (stream.sol() && stream.match(/^[0-9a-fA-F]{4,}:?/)) return "number";
      if (stream.sol() && stream.match(/^\*\s*$/)) return "comment";
      if (stream.match(/^\|[^|]*\|?/)) return "string";
      if (stream.eatSpace()) return null;
      // 헥사 바이트 (2자리 또는 xxd식 4자리 묶음)
      if (stream.match(/^[0-9a-fA-F]{2}(?:[0-9a-fA-F]{2})?(?=\s|$)/)) return null;
      // 그 외 비공백 덩어리 = ASCII 컬럼
      if (stream.match(/^\S+/)) return "string";
      stream.next();
      return null;
    },
  });

  // 펜스 언어 별칭: language-data에 없는 이름을 지원 언어로 매핑
  const FENCE_LANG_ALIASES: Record<string, string> = {
    armasm: "gas",
    "arm-asm": "gas",
    asm: "gas",
    hex: "hexdump",
  };

  const CUSTOM_FENCE_LANGS: Record<string, Language> = {
    tree: treeLanguage,
    hexdump: hexdumpLanguage,
  };

  function resolveFenceLanguage(info: string): { desc?: LanguageDescription; lang?: Language } | null {
    const name = (FENCE_LANG_ALIASES[info.toLowerCase()] ?? info).trim();
    if (!name) return null;
    const custom = CUSTOM_FENCE_LANGS[name.toLowerCase()];
    if (custom) return { lang: custom };
    const desc = LanguageDescription.matchLanguageName(languages, name, true);
    return desc ? { desc } : null;
  }

  // 프리뷰(비에디터)에서도 하이라이트 클래스가 동작하도록 스타일 모듈을 문서에 마운트
  // (에디터 안에서는 CM이 자동으로 마운트하지만, 프리뷰는 CM 밖에서 렌더링된다)
  if (innHighlight.module) {
    StyleModule.mount(document, innHighlight.module);
  }

  // 코드블록(``` 펜스) 줄에 배경을 입혀 옵시디언처럼 블록으로 보이게
  function buildCodeBlockDecorations(view: EditorView): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    for (const { from, to } of view.visibleRanges) {
      syntaxTree(view.state).iterate({
        from,
        to,
        enter: (node) => {
          if (node.name !== "FencedCode" && node.name !== "CodeBlock") return;
          const startLine = view.state.doc.lineAt(node.from).number;
          const endLine = view.state.doc.lineAt(node.to).number;
          for (let l = startLine; l <= endLine; l++) {
            const line = view.state.doc.line(l);
            const cls = "cm-codeblock-line"
              + (l === startLine ? " cm-codeblock-first" : "")
              + (l === endLine ? " cm-codeblock-last" : "");
            builder.add(line.from, line.from, Decoration.line({ class: cls }));
          }
          return false;
        },
      });
    }
    return builder.finish();
  }

  const codeBlockPlugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = buildCodeBlockDecorations(view);
      }
      update(u: ViewUpdate) {
        // 파스 트리는 백그라운드에서 완성되므로 트리 갱신도 감지해야
        // 최초 표시 시(스크롤 전)에도 코드블록이 그려진다
        if (u.docChanged || u.viewportChanged || syntaxTree(u.state) !== syntaxTree(u.startState)) {
          this.decorations = buildCodeBlockDecorations(u.view);
        }
      }
    },
    { decorations: (v) => v.decorations },
  );

  // --- 에디터 모드 인라인 이미지 (옵시디언 라이브 프리뷰 방식) ---
  // 항상 이미지로 표시. 이미지에 호버하면 </> 버튼이 나타나고,
  // 누르면 해당 이미지의 마크다운 코드가 잠깐 열린다 (커서가 벗어나면 다시 이미지).

  const revealImage = StateEffect.define<{ from: number; to: number } | null>();

  /** 버튼으로 "코드 보기"를 요청한 이미지 문법 범위 (편집을 따라 이동).
      커서가 범위를 벗어나는 순간 자동으로 해제된다 — 그래야 이후에
      주변을 클릭해도 코드가 다시 열리지 않는다. */
  const revealedImageRange = StateField.define<{ from: number; to: number } | null>({
    create: () => null,
    update(value, tr) {
      let v = value;
      if (v) {
        v = { from: tr.changes.mapPos(v.from), to: tr.changes.mapPos(v.to, 1) };
      }
      for (const e of tr.effects) {
        if (e.is(revealImage)) v = e.value;
      }
      if (v && tr.selection) {
        const head = tr.state.selection.main.head;
        if (head < v.from || head > v.to) v = null;
      }
      return v;
    },
  });

  class InlineImageWidget extends WidgetType {
    /** withButton=false: 코드가 열린 상태에서 문법 아래에 같이 보여주는 이미지 */
    constructor(
      readonly rawSrc: string,
      readonly alt: string,
      readonly resolved: string,
      readonly withButton: boolean,
      readonly matchLen: number,
    ) {
      super();
    }
    eq(other: InlineImageWidget) {
      return other.resolved === this.resolved
        && other.alt === this.alt
        && other.withButton === this.withButton
        && other.matchLen === this.matchLen;
    }
    toDOM(view: EditorView) {
      const wrap = document.createElement("span");
      wrap.className = "cm-inline-img-wrap";

      const img = document.createElement("img");
      img.className = "cm-inline-img";
      img.src = this.resolved;
      img.alt = this.alt;
      img.loading = "lazy";
      img.onerror = () => {
        const fallback = document.createElement("span");
        fallback.className = "cm-inline-img-broken";
        fallback.textContent = `![${this.alt}](${this.rawSrc})`;
        img.replaceWith(fallback);
      };
      wrap.append(img);

      if (this.withButton) {
        const btn = document.createElement("button");
        btn.className = "cm-img-code-btn";
        btn.type = "button";
        btn.textContent = "</>";
        btn.title = "Edit markdown";
        btn.addEventListener("mousedown", (e) => {
          e.preventDefault();
          e.stopPropagation();
          const pos = view.posAtDOM(wrap);
          view.dispatch({
            effects: revealImage.of({ from: pos, to: pos + this.matchLen }),
            selection: { anchor: pos },
          });
          view.focus();
        });
        wrap.append(btn);
      }
      return wrap;
    }
  }

  function buildImageDecorations(view: EditorView): { deco: DecorationSet; atomic: DecorationSet } {
    const deco = new RangeSetBuilder<Decoration>();
    // atomic에는 "접힌(이미지로 대체된)" 범위만 넣는다 —
    // 열린 코드의 색칠 mark까지 넣으면 글자 단위 편집이 막힌다
    const atomic = new RangeSetBuilder<Decoration>();
    const { state } = view;
    const revealed = state.field(revealedImageRange);
    const baseUrl = get(url);
    for (const { from, to } of view.visibleRanges) {
      const text = state.doc.sliceString(from, to);
      for (const m of text.matchAll(IMAGE_MD_RE)) {
        const start = from + m.index!;
        const end = start + m[0].length;
        // </> 버튼으로 연 이미지만 코드 표시 — 커서가 범위를 벗어나면
        // revealedImageRange가 스스로 해제된다.
        // (그 외에는 atomicRanges가 커서의 문법 내부 진입 자체를 막는다)
        const revealedHere = revealed != null && revealed.from <= end && revealed.to >= start;
        if (revealedHere) {
          // 열린 코드는 주황 계열로 구분해서 표시
          deco.add(start, end, Decoration.mark({ class: "cm-img-code-text" }));
          // 코드가 열린 동안에도 이미지는 문법 아래에 같이 표시
          deco.add(
            end,
            end,
            Decoration.widget({
              widget: new InlineImageWidget(m[2], m[1], resolveImageSrc(m[2], baseUrl), false, m[0].length),
              side: 1,
            }),
          );
        } else {
          const collapsed = Decoration.replace({
            widget: new InlineImageWidget(m[2], m[1], resolveImageSrc(m[2], baseUrl), true, m[0].length),
          });
          deco.add(start, end, collapsed);
          atomic.add(start, end, collapsed);
        }
      }
    }
    return { deco: deco.finish(), atomic: atomic.finish() };
  }

  const inlineImagePlugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      atomics: DecorationSet;
      constructor(view: EditorView) {
        ({ deco: this.decorations, atomic: this.atomics } = buildImageDecorations(view));
      }
      update(u: ViewUpdate) {
        if (u.docChanged || u.selectionSet || u.viewportChanged) {
          ({ deco: this.decorations, atomic: this.atomics } = buildImageDecorations(u.view));
        }
      }
    },
    {
      decorations: (v) => v.decorations,
      // 이미지로 접힌 문법은 커서가 내부로 들어갈 수 없는 하나의 단위로 취급
      // (주변 클릭/화살표 이동으로 코드가 튀어나오는 것 방지)
      provide: (plugin) =>
        EditorView.atomicRanges.of(
          (view) => view.plugin(plugin)?.atomics ?? Decoration.none,
        ),
    },
  );

  function scrollToHighlight() {
    tick().then(() => {
      if (highlightEl) {
        highlightEl.scrollIntoView({ block: "center", behavior: "smooth" });
      }
    });
  }

  // --- Shortcuts ---

  onMount(() => {
    registerAction("save", () => {
      if (editable) showDialog = true;
    });
    registerAction("exit-edit", () => {
      if (editable) tryExitEdit();
    });
  });

  // --- File operations ---

  async function getFileContent(filePath: string) {
    const seq = ++loadSeq;
    try {
      const content: string = await invoke("get_file_content", {
        filePath,
      });
      // 응답 대기 중 다른 파일로 전환됐으면 폐기 (다른 파일에 덮어쓰기 방지)
      if (seq !== loadSeq) return;
      fileContent = content;
      isConnected.set(true);
      syncEditorContent();
      // syncEditorContent의 dispatch가 updateListener를 태우므로 이후에 초기화
      isContentChanged = false;

      // 검색 결과에서 라인 점프 요청이 있으면 강조
      const pendingLine = get(gotoLine);
      if (pendingLine > 0) {
        highlightLine = pendingLine;
        gotoLine.set(0);
        scrollToHighlight();
      }
    } catch (error) {
      if (seq !== loadSeq) return;
      console.error("Failed to get file content", error);
      fileContent = "";
      syncEditorContent();
      isContentChanged = false;
      gotoLine.set(0);
      const connected: boolean = await invoke("check_connection");
      isConnected.set(connected);
      if (!connected) {
        addToast("SSH connection lost.");
      } else {
        addToast("Failed to load file.");
      }
    }
  }

  function startAutoSave() {
    if (!autoSaveTimer) {
      autoSaveTimer = setInterval(() => {
        saveContent(false, currentFilePath);
      }, autoSaveInterval);
    }
  }

  function stopAutoSave() {
    if (autoSaveTimer) {
      clearInterval(autoSaveTimer);
      autoSaveTimer = null;
    }
  }

  // "ok" = 저장+동기화 성공, "sync_failed" = 저장은 됐지만 이미지 동기화 실패,
  // "save_failed" = 저장 자체가 실패 (SSH 끊김 등)
  type SaveResult = "ok" | "sync_failed" | "save_failed";

  async function saveContent(manual: boolean = false, savePath?: string): Promise<SaveResult> {
    if (!manual && !isContentChanged) {
      return "ok";
    }
    // 에디터에서 최신 내용 가져오기
    if (view) {
      fileContent = view.state.doc.toString();
    }
    const genAtSave = changeGen;
    try {
      const syncOk = await invoke<boolean>("save_file_content", {
        filePath: savePath ?? $relativeFilePath,
        fileData: fileContent,
        manual,
      });
      // 저장 응답 대기 중 추가 입력이 없었을 때만 dirty 해제
      if (changeGen === genAtSave) {
        isContentChanged = false;
      }
      isConnected.set(true);
      lastSavedAt.set(new Date());
      return syncOk ? "ok" : "sync_failed";
    } catch (error) {
      console.error("Failed to save content:", error);
      const connected: boolean = await invoke("check_connection");
      isConnected.set(connected);
      if (!connected) {
        addToast("SSH connection lost.");
      } else {
        addToast("Failed to save file.");
      }
      return "save_failed";
    }
  }

  // --- Image paste ---

  function handlePaste(event: ClipboardEvent, cmView: EditorView): boolean {
    const items = event.clipboardData?.items;

    if (items) {
      const item = items[0];

      if (item.type.indexOf("image") !== -1) {
        event.preventDefault();
        // async 처리를 별도로 분리 — 동기적으로 true 반환해야 CM이 기본 동작 차단
        (async () => {
          try {
            const fileData = await readFileAsArrayBuffer(item.getAsFile()!);
            const currentPosition = cmView.state.selection.main.head;

            const uuidValue = uuidv4();
            const savedPath = await invoke("save_file_image", {
              filePath: currentFilePath,
              fileName: uuidValue,
              fileData: Array.from(fileData),
            });

            const insertText = `\n![${uuidValue}](${savedPath})`;
            cmView.dispatch({
              changes: { from: currentPosition, insert: insertText },
            });
            isContentChanged = true;
          } catch (e) {
            console.error("Image paste failed:", e);
            addToast("Failed to save image.");
          }
        })();
        return true;
      }
    }

    // 텍스트 붙여넣기: 외부 이미지 참조가 있으면 가로채서 sync 후 삽입
    const pastedText = event.clipboardData?.getData("text/plain");
    if (pastedText && hasExternalImageRefs(pastedText, currentFilePath)) {
      event.preventDefault();
      (async () => {
        try {
          const synced: string = await invoke("sync_pasted_refs", {
            filePath: currentFilePath,
            pastedText,
          });
          const pos = cmView.state.selection.main.head;
          cmView.dispatch({
            changes: { from: pos, insert: synced },
          });
          isContentChanged = true;
          addToast("Image links synced.", "success");
        } catch (e) {
          // sync 실패 시 원본 텍스트 그대로 삽입
          const pos = cmView.state.selection.main.head;
          cmView.dispatch({
            changes: { from: pos, insert: pastedText },
          });
          isContentChanged = true;
        }
      })();
      return true;
    }

    return false;
  }

  /** 텍스트에 외부 이미지 참조(다른 파일 이미지 또는 URL)가 있는지 확인 */
  function hasExternalImageRefs(text: string, filePath: string): boolean {
    const myPrefix = filePath.replace(/^\//, "") + "/";
    const patterns = [
      /!\[[^\]]*\]\(([^)]+)\)/g,           // ![alt](path)
      /<img\s[^>]*src\s*=\s*["']([^"']+)["']/g,  // <img src="path">
    ];
    for (const re of patterns) {
      let m;
      while ((m = re.exec(text)) !== null) {
        const path = m[1].trim();
        if (path.startsWith("http://") || path.startsWith("https://")) {
          return true;
        }
        const clean = path.replace(/^\//, "");
        if (!clean.startsWith(myPrefix)) {
          return true;
        }
      }
    }
    return false;
  }

  async function readFileAsArrayBuffer(file: File): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        if (reader.result instanceof ArrayBuffer) {
          resolve(new Uint8Array(reader.result));
        } else {
          reject(new Error("File reading resulted in null"));
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsArrayBuffer(file);
    });
  }

  onDestroy(() => {
    stopAutoSave();
    if (view) { view.destroy(); view = null; }
    unregisterAction("save");
    unregisterAction("exit-edit");
  });
</script>

<SavePopup show={showDialog}
  closeSave={() => {
    showDialog = false;
    editable = true;
  }}
  handleSave={async () => {
    const result = await saveContent(true, currentFilePath);
    showDialog = false;
    if (result === "ok") {
      await getFileContent(currentFilePath);
      addToast("File saved.", "success");
      editable = false;
    } else if (result === "sync_failed") {
      addToast("File saved, but image sync failed.", "info");
    }
    // save_failed: saveContent가 이미 원인 토스트를 띄움 — 편집 모드 유지
  }}
/>

<SavePopup show={showUnsavedDialog}
  title="Unsaved Changes"
  message="You have unsaved changes."
  saveLabel="Save"
  discardLabel="Discard"
  cancelLabel="Cancel"
  closeSave={handleUnsavedCancel}
  handleSave={handleUnsavedSave}
  handleDiscard={handleUnsavedDiscard}
/>

<!-- 콘텐츠 영역은 D2Coding 16px 유지 (UI 크롬과 분리)
     flex-1 + min-h-0: TopBar를 제외한 남은 공간만 차지 (h-full이면
     TopBar 높이만큼 넘쳐 상태바를 뚫고 이중 스크롤이 생긴다) -->
<div class="relative flex-1 min-h-0 w-full">
{#if currentFilePath && fileContent.includes("```")}
  <button
    class="code-wrap-btn"
    class:active={codeWrap}
    title={codeWrap ? "코드블록: 줄바꿈 (클릭 시 횡스크롤)" : "코드블록: 횡스크롤 (클릭 시 줄바꿈)"}
    on:click={() => (codeWrap = !codeWrap)}
  >
    {codeWrap ? "Wrap: on" : "Wrap: off"}
  </button>
{/if}
<div bind:this={contentDiv}
  class="{editable ? 'overflow-hidden' : 'overflow-y-auto'} h-full w-full content-root"
  class:code-wrap={codeWrap}
  style="font-family: var(--font-mono); font-size: 16px; line-height: 24px;">
  {#if editable}
    <div bind:this={editorContainer} class="h-full w-full"></div>
  {:else if !currentFilePath}
    <!-- 빈 상태: 파일 미선택 -->
    <div class="empty-state">
      <div class="empty-logo"><LogoSVG /></div>
      <p class="empty-title">Select a file from the sidebar to start writing</p>
      <div class="empty-hints">
        <span><kbd>Double-click</kbd> edit</span>
        <span><kbd>Ctrl+S</kbd> save</span>
        <span><kbd>F2</kbd> rename</span>
        <span><kbd>Esc</kbd> exit edit</span>
      </div>
    </div>
  {:else}
    <div
      tabindex="0"
      role="button"
      class="break-all w-full min-h-full whitespace-pre-wrap p-4"
      style="border: 1px solid transparent; border-radius: 8px;"
      on:dblclick={handlePreviewDblClick}
    >
      {#each toPreviewBlocks(fileContent) as block}
        {#if block.kind === "code"}
          {@const inner = codeInnerLines(block)}
          {@const tokenLines = highlightTokens(block, hlVersion)}
          <!-- 펜스(```) 줄은 숨기고 내부 코드만 표시. 컨테이너의 data-line은
               여백 클릭 시 더블클릭 편집이 문서 맨 위로 튀지 않게 하는 폴백 -->
          <div class="preview-code-block" class:wrap={codeWrap}
            data-line={inner[0]?.i ?? block.lines[0].i}>
            {#each inner as pl, idx}
              {@const toks = tokenLines[idx] ?? [{ text: pl.text, cls: null }]}
              {#if highlightLine === pl.i + 1}
                <div data-line={pl.i} class="highlight-line" bind:this={highlightEl}>{#each toks as t}{#if t.cls}<span class={t.cls}>{t.text}</span>{:else}{t.text}{/if}{:else}{' '}{/each}</div>
              {:else}
                <div data-line={pl.i}>{#each toks as t}{#if t.cls}<span class={t.cls}>{t.text}</span>{:else}{t.text}{/if}{:else}{' '}{/each}</div>
              {/if}
            {/each}
          </div>
        {:else}
          {@const mdLines = markdownTokens(block.lines.map((l) => l.text).join("\n"))}
          {#each block.lines as pl, idx}
            {@const hasImg = pl.segments.some((s) => s.type === "img")}
            {@const toks = mdLines[idx] ?? []}
            {#if highlightLine === pl.i + 1}
              <div data-line={pl.i} class="highlight-line" bind:this={highlightEl}>{#if hasImg}{#each pl.segments as seg}{#if seg.type === "img"}<img class="preview-img" src={resolveImageSrc(seg.src, $url)} data-raw-src={seg.src} alt={seg.alt} loading="lazy" on:error={onPreviewImgError} />{:else}{seg.text}{/if}{/each}{:else}{#each toks as t}{#if t.cls}<span class={t.cls}>{t.text}</span>{:else}{t.text}{/if}{:else}{' '}{/each}{/if}</div>
            {:else}
              <div data-line={pl.i}>{#if hasImg}{#each pl.segments as seg}{#if seg.type === "img"}<img class="preview-img" src={resolveImageSrc(seg.src, $url)} data-raw-src={seg.src} alt={seg.alt} loading="lazy" on:error={onPreviewImgError} />{:else}{seg.text}{/if}{/each}{:else}{#each toks as t}{#if t.cls}<span class={t.cls}>{t.text}</span>{:else}{t.text}{/if}{:else}{' '}{/each}{/if}</div>
            {/if}
          {/each}
        {/if}
      {/each}
    </div>
  {/if}
</div>
</div>

<style>
  .code-wrap-btn {
    position: absolute;
    top: 0.5rem;
    right: 1.25rem;
    z-index: 10;
    padding: 2px 10px;
    font-size: 11px;
    font-family: var(--font-ui);
    background-color: var(--popup-bg-color);
    color: var(--reverse-secondary-color);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    box-shadow: none;
    opacity: 0.75;
  }
  .code-wrap-btn:hover {
    opacity: 1;
  }
  .code-wrap-btn.active {
    background-color: var(--accent-tint);
    color: var(--accent-strong);
    border-color: var(--accent-color);
  }

  /* 프리뷰 코드블록: 하나의 블록으로 감싸고 기본은 횡스크롤 */
  .preview-code-block {
    background-color: var(--code-block-bg);
    border: 1px solid var(--code-block-border);
    border-radius: 8px;
    padding: 0.5rem 0.75rem;
    margin: 0.25rem 0;
    overflow-x: auto;
  }
  .preview-code-block > div {
    white-space: pre;
    width: max-content;
    min-width: 100%;
  }
  .preview-code-block.wrap > div {
    white-space: pre-wrap;
    width: auto;
  }

  /* 에디터 코드블록: 기본 횡스크롤(줄바꿈 없음), .code-wrap이면 줄바꿈 */
  .content-root :global(.cm-codeblock-line) {
    white-space: pre;
  }
  .content-root.code-wrap :global(.cm-codeblock-line) {
    white-space: pre-wrap;
  }

  .empty-state {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--reverse-secondary-color);
    user-select: none;
  }
  .empty-logo {
    opacity: 0.25;
    margin-bottom: 0.5rem;
  }
  .empty-title {
    opacity: 0.7;
  }
  .empty-hints {
    display: flex;
    gap: 1.25rem;
    flex-wrap: wrap;
    justify-content: center;
    opacity: 0.55;
    font-size: 12px;
  }
  .empty-hints kbd {
    font-family: var(--font-ui);
    font-size: 11px;
    padding: 1px 5px;
    border: 1px solid var(--border-color);
    border-bottom-width: 2px;
    border-radius: 4px;
    background-color: var(--secondary-color);
    margin-right: 4px;
  }

  .preview-img {
    display: block;
    max-width: min(100%, 720px);
    max-height: 60vh;
    margin: 0.5rem 0;
    border-radius: 8px;
    border: 1px solid var(--border-color);
  }
  :global(.preview-img-broken) {
    opacity: 0.55;
  }

  .highlight-line {
    background-color: var(--search-match-bg, rgba(255, 213, 79, 0.25));
    border-radius: 2px;
    margin: 0 -0.25rem;
    padding: 0 0.25rem;
  }
</style>
