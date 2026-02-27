# PTY Terminal Vim Freeze Fix (Release Only)

> 상태: 해결  
> 적용 버전: v1.1.2 hotfix  
> 범위: Windows + Tauri 2 + WebView2 + xterm.js + SSH PTY

## 1. 문제 요약

릴리즈 빌드에서 SSH 터미널로 `vim`에 진입하면 화면은 정상인데 키 입력(`i`, `h/j/k/l` 등)이 반응하지 않는 문제가 있었다.  
dev 모드에서는 같은 동작이 정상이었다.

초기에는 IPC, PTY, 단축키, 렌더링 등 광범위한 후보를 의심했지만, 최종적으로는 **vim alt-screen 전환 직후의 입력 경로 타이밍 불안정**과 **입력 전송 경로의 중복/경합**이 핵심이었다.

---

## 2. 증상 상세

- 릴리즈 모드에서만 재현.
- `vim` 화면 렌더링/색상은 정상.
- 영어 키 입력만 무반응인 경우가 있었고, IME 조합 관련 입력은 혼선이 생김.
- 앱 실행 직후 빠르게 `vim` 진입 시 실패 확률이 높았고, 앱을 한동안 켜둔 후 진입 시 성공하는 패턴이 관찰됨.

이 패턴은 “입력 경로가 완전히 죽음”이 아니라 “초기 타이밍 구간 불안정”을 시사한다.

---

## 3. 아키텍처 컨텍스트

### 3-1. 입력 경로

`xterm textarea -> terminal.onData -> invoke(write_pty_cmd) -> Rust mpsc -> ssh channel.write`

### 3-2. 출력 경로

`ssh channel.read -> on_output -> Tauri Channel<String> -> onmessage -> terminal.write`

### 3-3. 관련 파일

- `src/sidebar/TerminalPopup.svelte`
- `src/sidebar/terminal/TerminalInputController.ts` (이번 리팩토링에서 신규)
- `src-tauri/src/commands/pty_command.rs`
- `src-tauri/src/services/pty_service.rs`

---

## 4. 이전 시도와 한계

아래 시도들은 부분 개선이 있거나 무효였고, 근본 해결로 이어지지 못했다.

1. 글로벌 단축키 우회/비활성화  
   - 키 이벤트 경로와 직접 관련은 낮았고, 증상 본질 해결 실패.
2. 포커스 보강(`terminal.focus`)만 단독 적용  
   - 일부 환경에서 개선처럼 보였지만 재현성 있는 해결이 아님.
3. 출력 배칭(rAF/queue)  
   - 과도한 복잡도 대비 안정 효과 제한.
4. Rust 쪽 read/write 배칭 조정  
   - 병목 구간의 본질이 프론트 입력 타이밍/이벤트 해석 측에 있어 한계.

---

## 5. 근본 원인 정리

문제는 단일 원인이라기보다 다음의 조합이었다.

1. 릴리즈(WebView2/custom protocol/최적화)에서 `vim` alt-screen 초기 구간의 입력 해석 타이밍이 불안정.
2. 기존 `onData`의 타이머 기반(2ms) 전송은 초기 구간에서 지연/순서 경합을 만들 가능성이 큼.
3. fallback 입력 경로를 추가하면, `onData`와 중복 전송이 발생해 키가 2회 입력되는 부작용이 생길 수 있음.

따라서 해법은 “한 경로만 더 빠르게 보내기”가 아니라, **모드별 입력 경로를 명시적으로 분리하고 단일 전송 큐로 일원화**하는 구조가 필요했다.

---

## 6. 최종 해결 전략

### 6-1. 핵심 아이디어

- `alt-screen(vim)` 구간과 일반 쉘 구간을 분리 처리.
- 입력 전송은 단일 큐/단일 in-flight로 직렬화.
- alt-screen 구간에서 fallback이 동작할 때는 `onData` 경로를 차단해 중복 전송 방지.

### 6-2. 실제 적용

1. `terminal.open()` 직후 즉시 `terminal.focus()`.
2. `setTimeout(2ms)` 배칭 제거, `queueMicrotask` + single in-flight 전송으로 변경.
3. alt-screen(`ESC[?1049h` ~ `ESC[?1049l`) 감지 후 키 fallback 적용.
4. alt-screen 중에는 `onData`를 무시해 중복 입력 차단.

---

## 7. 리팩토링 (구조 개선)

이번 수정은 문제만 임시 봉합하지 않고 입력 계층을 분리했다.

### 7-1. 신규 컴포넌트

`src/sidebar/terminal/TerminalInputController.ts`

역할:

- alt-screen 상태 관리
- custom key fallback 처리
- `onData` 수신 처리
- 입력 버퍼링/flush/직렬 전송
- 중복 전송 방지 규칙 통합

### 7-2. TerminalPopup 책임 축소

`TerminalPopup.svelte`는 다음 역할만 유지:

- xterm 생성/테마/렌더링
- PTY start/resize/stop IPC
- 서버 출력 수신 후 화면 렌더
- 입력 컨트롤러 생성/dispose

즉, 기존 “UI + 입력 상태기계 + 전송 큐 + fallback 로직”이 한 파일에 섞여 있던 구조를 분리했다.

### 7-3. 기대 효과

- 입력 관련 버그가 `TerminalInputController`로 국소화됨.
- 향후 `nvim`, `less`, `man` 등 TUI별 특수 처리 확장 시 영향 범위가 작아짐.
- “키 2회 입력”, “초기 타이밍 실패” 같은 회귀를 구조적으로 예방.

---

## 8. 검증 결과

릴리즈 빌드 기준:

- 앱 실행 직후 빠른 `vim` 진입: 정상
- 일정 시간 후 `vim` 진입: 정상
- 키 입력: 1회 입력당 1회 전달(중복 없음)
- `i`, `hjkl`, `Esc`, 화살표 등 기본 동작 정상

---

## 9. 남은 리스크 / 후속 작업

1. fallback 키 매핑은 현재 일반/내비게이션 중심이다.  
   - 필요 시 기능키(F1~F12), 조합키 범위를 단계적으로 확장.
2. alt-screen 판별은 escape 시퀀스 기반이다.  
   - 일부 터미널 앱에서 다른 전환 시퀀스를 쓰는 경우 확장 필요.
3. 회귀 방지 자동화가 아직 부족하다.  
   - 추천: `vim` 진입 직후 입력 E2E 시나리오 추가.

---

## 10. 결론

이번 이슈는 릴리즈 환경의 입력 타이밍 불안정과 경로 경합이 겹친 문제였다.  
해결은 단순 튜닝이 아니라 입력 파이프라인을 명확히 분리/직렬화하는 구조 개편으로 달성했다.

