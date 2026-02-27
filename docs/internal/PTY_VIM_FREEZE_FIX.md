# PTY 터미널 vim 멈춤 현상 — 진단 기록

> **상태**: **해결**
> **버전**: v1.1.2
> **현상**: 릴리즈 빌드에서 vim 실행 시 영어 키 입력이 안 됨 (화면 렌더링 자체는 정상)
> **환경**: Windows 10/11, Tauri 2.x, xterm.js v6 (`@xterm/xterm ^6.0.0`), WebView2, ssh2 crate

---

## 1. 프로젝트 컨텍스트

Tauri 2 데스크톱 앱에서 SSH를 통해 원격 서버에 PTY 세션을 열고, 프론트엔드에서 xterm.js로 터미널을 렌더링하는 구조.

### 데이터 흐름

```
[키 입력] → xterm.js textarea → onData() → invoke("write_pty_cmd") → Rust mpsc → SSH channel.write()
[화면 출력] → SSH channel.read() → on_output() → Tauri Channel<String> IPC → onmessage → terminal.write()
```

### 핵심 파일

| 파일 | 역할 |
|---|---|
| `src/sidebar/TerminalPopup.svelte` | 터미널 팝업 UI. xterm.js 생성, IPC 수신, 키 입력 전송 |
| `src-tauri/src/services/pty_service.rs` | SSH PTY I/O 루프. 비블로킹 read/write, mpsc 메시지 처리 |
| `src-tauri/src/commands/pty_command.rs` | Tauri IPC 핸들러. Channel<String>으로 output 콜백 연결 |
| `src/shortcut.ts` | 글로벌 키보드 단축키 (Escape, Enter, F2, Ctrl+S) |

### dev vs release 차이점

| | dev | release |
|---|---|---|
| Rust 최적화 | 없음 (debug) | `-O2` (빠른 I/O 루프) |
| Windows 서브시스템 | console | `windows` (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`) |
| 프론트엔드 URL | `http://localhost:5573` (Vite dev server) | `https://tauri.localhost/` (custom protocol) |
| JS 번들링 | Vite HMR, 소스맵 | minified bundle |

---

## 2. 증상 (100% 재현)

- SSH 터미널에서 **vim 실행 시** 영어 키 입력이 안 됨 (`i`, `hjkl` 등)
- 한글 입력은 됨 (Windows IME 조합 이벤트가 별도 경로)
- vim 화면 렌더링 자체는 정상 (색상 깨짐 없음). 다만 키 입력에 대한 화면 반응이 없음
- **릴리즈 모드에서만** 발생. dev 모드에서는 정상
- 일반 셸(bash)에서는 릴리즈에서도 정상. `Ctrl+D`, 명령어 실행 등 문제 없음
- vim 진입 **즉시** 발생 (시간 경과와 무관)
- 포커스 시 커서가 흰색 블록으로 채워지고, 포커스 해제 시 빈 사각형 → xterm.js 포커스 처리는 정상
- 한글이 vim 커서 위치에 표시됨 → **IME 조합 오버레이**일 가능성 높음 (vim이 실제로 처리한 게 아님)

---

## 3. 진단에서 확인된 사실

### 입력 경로: 완전히 정상

1. **xterm.js가 키를 받음**: `svelte:window on:keydown`에 toast 추가 → 터미널 포커스 시 일반 키는 toast 안 뜸 (xterm.js가 `stopPropagation`으로 소비). Shift만 뜸.
2. **onData 정상 발동**: `terminal.onData()` 콜백에 toast 추가 → **키를 누르면 onData가 발생**. 키 데이터가 PTY까지 전달됨.
3. **결론**: 키보드 → xterm.js → onData → invoke("write_pty_cmd") → Rust → SSH → vim 경로는 정상.

### IPC 데이터: 손상 없음

4. **쉘 프롬프트 hex dump** (첫 ESC 포함 메시지):
```
1b 5b 3f 32 30 30 34 68  →  ESC[?2004h (bracketed paste)
1b 5d 30 3b 64 6f 6e 67  →  ESC]0;dong... (window title)
6b 69 6d 40 6e 6f 64 65  →  kim@node...
1b 5b 30 31 3b 33 32 6d  →  ESC[01;32m (bold green)
```
→ ESC 시퀀스(`0x1b`)가 정상 전달. JSON 직렬화/역직렬화에서 제어 문자 손상 없음.

5. **vim 초기화 hex dump** (154바이트, `ESC[?1049h` 감지):
```
1b 5b 3f 31 30 30 30 68  →  ESC[?1000h (마우스 트래킹)
1b 5b 3f 31 30 34 39 68  →  ESC[?1049h (대체 화면 버퍼)
1b 5b 32 32 3b 30 3b 30 74  →  ESC[22;0;0t (윈도우 타이틀 저장)
1b 5b 3e 34 3b 32 6d     →  ESC[>4;2m (modifyOtherKeys level 2)
1b 5b 3f 31 68           →  ESC[?1h (앱 커서 키)
1b 3d                    →  ESC= (앱 키패드)
1b 5b 3f 32 30 30 34 68  →  ESC[?2004h (bracketed paste)
1b 5b 3f 31 30 30 34 68  →  ESC[?1004h (포커스 리포팅)
1b 5b 31 3b 32 38 72     →  ESC[1;28r (스크롤 영역 1-28)
```
→ vim 초기화 이스케이프 시퀀스가 **정확하게** 프론트엔드에 도달.

### 터미널 크기: 정상

6. **터미널**: 103 cols × 28 rows, **컨테이너**: 880 × 461 px → 정상 크기.

### 글로벌 단축키: 무관

7. `shortcut.ts`의 `handleShortcutEvent` 첫 줄에 `return;` 삽입하여 전부 비활성화 → **여전히 안 됨**.

---

## 4. 시도한 수정 및 결과

### 4-1. 글로벌 단축키에 xterm 바이패스 추가 → 효과 없음
- `handleShortcutEvent`에서 `event.target`이 `.xterm` 내부면 return
- 결과: 변화 없음. 이후 단축키 전체 비활성화 테스트로 완전히 배제.

### 4-2. 포커스 관리 → 효과 없음
- `terminal.focus()` 호출, mousedown 클릭 핸들러, focus recovery 추가
- 결과: 변화 없음.

### 4-3. window keydown에서 PTY로 직접 키 전송 → 효과 없음
- xterm.js를 완전히 우회해서 window keydown → invoke("write_pty_cmd") 직접 전송
- 결과: xterm.js가 키를 소비하여(stopPropagation) window handler에 도달하지 않음.

### 4-4. 프론트엔드 출력 rAF 배칭 → 효과 없음
- `terminal.write()` 호출을 `requestAnimationFrame`으로 프레임당 1회로 제한
- 이론: 릴리즈에서 빠른 I/O 루프가 수백 개 IPC 메시지 → 수백 번 write() → 파서 상태 손상
- 결과: 배칭 적용해도 **동일 현상**. write 빈도가 원인이 아님.
- **유저가 되돌림**: 현재 코드는 직접 `terminal?.write(data)`.

### 4-5. Rust I/O 루프 출력 배칭 (16ms/8KB/idle) → 효과 없음
- `output_buf`에 축적 후 idle/size/time 기반 flush. IPC 메시지 수를 초당 ~60개로 제한.
- 결과: **동일 현상**. Rust 쪽 메시지 빈도도 원인이 아님.
- **유저가 되돌림**: 현재 코드는 read마다 즉시 `on_output()` 호출.

### 4-6. modifyOtherKeys (`ESC[>4;2m`) 필터링 → 효과 없음
- vim이 보내는 `ESC[>4;2m` (modifyOtherKeys level 2)을 프론트엔드에서 정규식으로 제거
- 이론: xterm.js가 이 모드를 잘못 구현하면 키 인코딩이 달라져 vim이 못 알아들음
- 결과: **동일 현상**.
- **유저가 되돌림**.

---

## 5. 현재 코드 상태

프론트엔드(`TerminalPopup.svelte`): 배칭 없이 직접 write. 입력은 2ms 배칭.
```typescript
// 출력: 직접 write
onEvent.onmessage = (data: string) => {
  if (data === "\x00__PTY_CLOSED__") { stopTerminal(); closeTerminal(); return; }
  terminal?.write(data);
};

// 입력: 2ms 배칭
terminal.onData((data: string) => {
  inputBuffer += data;
  if (!inputTimer) {
    inputTimer = window.setTimeout(() => {
      const buf = inputBuffer;
      inputBuffer = "";
      inputTimer = null;
      invoke("write_pty_cmd", { data: buf }).catch(console.error);
    }, 2);
  }
});
```

백엔드(`pty_service.rs`): drain + non-blocking write + smart sleep 유지. 출력 배칭은 없음 (read마다 즉시 on_output).

---

## 6. 핵심 미스터리

**왜 릴리즈에서만?** 모든 진단에서 dev와 release의 데이터 경로에 차이가 없음:
- IPC 데이터 동일 (hex dump 확인)
- 터미널 크기 동일
- 코드 동일 (조건 분기 없음)

dev와 release의 실제 차이는 코드가 아닌 **환경**:
1. `windows_subsystem = "windows"` — Windows 메시지 루프/윈도우 스타일 변경
2. `https://tauri.localhost/` custom protocol — WebView2의 origin/보안 컨텍스트 변경
3. Vite bundled JS — 동일한 코드지만 minified/bundled 형태
4. Rust `-O2` 최적화 — I/O 루프 실행 속도 차이 (그러나 배칭으로 정규화해도 동일)

---

## 7. 미탐구 영역 (다음 조사 방향)

### 가장 유력한 후보

**A. WebView2 custom protocol과 xterm.js 호환성**
- `https://tauri.localhost/`에서 xterm.js의 canvas 렌더러가 다르게 동작할 가능성
- xterm.js가 내부적으로 blob URL이나 Web Worker를 사용하는 경우 custom protocol CSP에 의해 차단될 수 있음
- 테스트 방법: `tauri.conf.json`에서 `"dangerousDisableAssetCspModification": true` 설정 후 CSP를 완전히 열어서 테스트

**B. `windows_subsystem = "windows"`의 영향**
- 이 플래그가 WebView2의 키보드 이벤트 디스패치에 영향을 줄 수 있음
- 테스트 방법: 릴리즈 빌드에서 `#![cfg_attr(...)]`를 일시 제거하고 `windows_subsystem = "console"`로 빌드하여 비교

**C. Tauri Channel vs Event 시스템**
- `Channel<String>`이 릴리즈 모드의 custom protocol IPC 경로에서 메시지 전달 타이밍이 다를 수 있음
- `channel.send()`는 성공하지만 프론트엔드에 도착이 지연/누락될 수 있음
- 테스트 방법: Channel 대신 `app_handle.emit("pty-output", &text)` + `listen()` 방식으로 변경하여 비교

**D. xterm.js의 `terminal.write()` 내부 비동기 처리**
- xterm.js v5+의 `write()`는 내부적으로 비동기 (write buffer 사용)
- `write(data, callback)` 형태로 각 write 완료를 기다려보기
- 테스트 방법: write queue + callback 패턴 구현

**E. xterm.js canvas 렌더러 대체**
- canvas 렌더러가 WebView2 custom protocol 환경에서 문제가 있을 수 있음
- 테스트 방법: `@xterm/addon-webgl` 또는 DOM 렌더러로 교체하여 비교

### 보조 후보

**F. base64 인코딩 테스트**
- IPC hex dump가 정상이라 가능성 낮지만, Rust에서 base64 인코딩 → JS에서 디코딩 → `terminal.write(Uint8Array)`로 문자열 경로 완전 우회

**G. 로컬 PTY로 비교**
- SSH가 아닌 로컬 PTY (Windows ConPTY)로 vim 실행 시 동일 현상이면 SSH와 무관
- 다만 이 앱은 원격 서버 전용이라 구현 비용 높음

**H. xterm.js 버전 업그레이드/다운그레이드**
- `@xterm/xterm ^6.0.0`의 특정 버전 버그일 수 있음

---

## 8. 재현 방법

```bash
cd im-not-notion
npm run tauri build          # 릴리즈 빌드
# 빌드된 .msi 설치 또는 직접 실행
# 앱에서 서버 연결 → 터미널 열기 → vim 실행
# → 영어 키 입력 안 됨, 색상 깨짐 확인
# → dev 모드 (npm run tauri dev)에서는 동일 조작이 정상 동작
```

---

## 9. 참고: Tauri + xterm.js 알려진 이슈 (웹 조사)

- **Tauri CSP 자동 주입**: 릴리즈 빌드 시 nonce/hash 주입으로 동적 스크립트 차단 가능 ([Tauri #3583](https://github.com/tauri-apps/tauri/issues/3583))
- **WebView2 WebGL 성능 저하**: 릴리즈에서 WebGL 렌더링 느려짐 ([Tauri #8020](https://github.com/tauri-apps/tauri/issues/8020))
- **IPC custom protocol 실패**: `ipc://` 스킴이 CSP에 의해 차단, postMessage 폴백 ([Tauri #12835](https://github.com/tauri-apps/tauri/issues/12835))
- **xterm.js fitAddon 리사이즈 이슈**: Svelte + Tauri 환경에서 fit() 호출 시 데이터 손실 ([xterm.js #3887](https://github.com/xtermjs/xterm.js/issues/3887))

---

## 10. 최종 해결 내용 (v1.1.2 hotfix)

### 10-1. 실제 원인 패턴 (현상 재해석)

- 사용자 관찰: **앱 실행 직후 바로 `vim` 진입 시 실패**, 앱을 켜둔 뒤 시간이 지난 후 진입하면 입력 성공.
- 이 패턴은 "경로 자체가 완전히 죽음"이 아니라, **`vim` 초기 진입(alt screen 전환 직후) 타이밍의 입력 처리 불안정**을 시사.
- 특히 릴리즈(WebView2 + custom protocol + 최적화)에서만 재현되는 점과 일치.

### 10-2. 적용한 최종 수정

`src/sidebar/TerminalPopup.svelte`에 다음을 적용:

1. `terminal.open()` 직후 `terminal.focus()`를 즉시 호출해 초기 포커스 안정화.
2. 입력 전송의 `setTimeout(2ms)` 배칭 제거:
   - `queueMicrotask` + single in-flight `invoke("write_pty_cmd")` 직렬화로 변경.
3. `vim` alt screen 구간(`ESC[?1049h` ~ `ESC[?1049l`)에 한해 입력 fallback 적용:
   - `attachCustomKeyEventHandler`에서 키를 직접 `write_pty_cmd`로 전달.
   - 일반 문자/Enter/Backspace/Tab/Escape/Arrow/Home/End/Delete/PageUp/PageDown 처리.
   - `Process`/`Unidentified` 키는 `KeyboardEvent.code` 기반 ASCII 복원 시도.
4. 중복 입력 방지:
   - alt screen 활성 중에는 `terminal.onData` 경로를 건너뛰어,
   - fallback 전송과 onData 전송이 동시에 발생하지 않도록 차단.

### 10-3. 검증 결과

- 릴리즈 빌드에서 **즉시 `vim` 진입 시에도 입력 정상 동작**.
- `i`, `hjkl` 등 기본 입력 정상.
- 초기 수정에서 발생했던 "키가 2번 입력" 문제는 alt screen 중 `onData` 차단으로 해결.
- 사용자 확인: 현재 버전에서 빠른 진입/늦은 진입 모두 정상, 키 1회 입력 정상.

### 10-4. 후속 참고

- 본 수정은 `vim` 계열(TUI 편집기) 초기 진입 안정성 보강 목적.
- 추후 필요 시 fallback 조건을 `vim`/`nvim` 감지 기반으로 더 좁히는 리팩터링 가능.
