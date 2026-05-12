# Plugin System

> im-not-notion 플러그인 시스템 — 서버에서 스크립트를 실행하고 NDJSON 양방향 프로토콜로 호스트와 통신.

핵심 원칙:
- **서버 실행**: 콘텐츠가 있는 서버에서 스크립트가 돌아 SFTP 왕복을 줄임
- **언어 무관**: shebang으로 인터프리터 지정 (Python/bash/Node 등). 현재 모든 플러그인은 Python.
- **NDJSON 양방향 프로토콜**: stdin/stdout으로 JSON 메시지를 줄 단위로 주고받음
- **UI는 호스트가 제공**: 플러그인은 메시지만 emit, 화면은 im-not-notion이 그림
- **느슨한 결합**: 플러그인이 호스트 코드를 import하지 않음 — 의존 대상은 JSON 스키마뿐

---

## 디렉토리 구조

```
서버:
~/.inn_plugins/
├── <plugin-name>/
│   ├── plugin.json        # 매니페스트
│   ├── main.py            # 진입점 (shebang 포함, chmod +x)
│   └── .disabled          # (선택) 존재하면 비활성화 상태
└── ...

로컬 (개발 시):
<plugin_local_path>/
├── <plugin-name>/
│   ├── plugin.json
│   └── main.py
```

`im-not-notion-plugins/` 레포가 메인 앱 레포의 git submodule 후보. 실제 운영에서는 로컬 path 설정 → 앱에서 install/pull로 서버와 동기화.

---

## plugin.json 스펙

```jsonc
{
  "name": "wayback",                                  // 유일한 식별자
  "description": "Archive external URLs ...",
  "version": "2.0.0",
  "entry": "main.py",                                 // 실행 가능한 진입점
  "triggers": [
    {
      "type": "manual",
      "content": {
        "label": "Archive URL",
        "input": [
          { "name": "url",    "type": "text",    "label": "URL",     "default": "" },
          { "name": "title",  "type": "text",    "label": "Title",   "default": "" },
          { "name": "tags",   "type": "text",    "label": "Tags",    "default": "" }
        ],
        "shortcut": "Ctrl+Shift+A"                    // (선택) 단축키
      }
    },
    { "type": "manual", "content": { "label": "Manage Archives", "input": [] } },

    { "type": "cron", "content": {
        "schedule": "0 19 * * 0",                     // 표준 cron 표현식
        "label": "Weekly backup",
        "priority": 50                                // (선택) 동일 schedule 내 순서
    }},

    { "type": "hook", "content": {
        "event": "AfterFileSave",                     // HookEvent enum 값
        "priority": 99                                // (선택, 기본 50) 낮을수록 먼저
    }}
  ]
}
```

### Trigger 타입

| 타입 | 사용 시점 | 우선순위 적용 |
|---|---|---|
| `manual` | 사용자가 PluginPanel에서 클릭 | — |
| `cron` | 서버 crontab이 정해진 시각에 호출 (앱 불필요) | 동일 schedule 내 |
| `hook` | 백엔드 이벤트(파일 저장/이동/삭제/생성)에 자동 실행 | 동일 event 내 |

### Hook 이벤트

`types/plugin.rs::HookEvent`:
- `AfterFileSave` — 수동 저장(Ctrl+S) 시 이미지 동기화 직후
- `AfterFileMove` — 파일/폴더 이동·이름변경 commit 직후
- `AfterFileDelete` — 파일 삭제 직후
- `AfterFileCreate` — 새 콘텐츠 생성 직후

### InputField 타입 (현재 지원)

- `text` — 일반 텍스트 입력 (PluginInputPopup에서 `<input type="text">`)
- `password` — 마스킹 텍스트 입력
- `boolean` — 체크박스

---

## NDJSON 프로토콜

각 메시지는 **한 줄의 JSON + `\n`**. 플러그인은 stdin에서 메시지를 읽고 stdout으로 메시지를 보냄. stderr는 호스트가 읽지 않음 (debug용으로만 사용).

### 흐름 (Manual 트리거 + 동적 prompt 케이스)

```
Host                              Plugin
 │                                  │
 │  {"type":"input", ...}\n  ────▶  │  stdin.readline()
 │                                  │  ... 처리 ...
 │  ◀────  {"type":"progress",...}\n   stdout.write+flush
 │  → emit Tauri event              │  ... 더 처리 ...
 │  ◀────  {"type":"prompt",...}\n     stdout.write+flush
 │  → emit + recv_timeout(600s)     │  stdin.readline() (블록)
 │                                  │
 │ (user responds via IPC)          │
 │  {"type":"prompt_response",...}\n ─▶│
 │                                  │  ... 응답 처리 ...
 │  ◀────  {"type":"result",...}\n     stdout.write+flush, exit
 │  → 최종 PluginResult로 반환     │
```

### Host → Plugin 메시지

```json
// 초기 입력 (항상 첫 줄, 1회)
{
  "type": "input",
  "trigger": "manual",                    // "manual" | "cron" | "hook"
  "url": "...", "title": "...", "tags": "...",   // manual: 입력 폼 필드 그대로 펼침
  "event": "AfterFileSave",               // hook: 이벤트 이름
  "data": { ... },                        // hook: 이벤트별 데이터 (예: {src, dst})
  "context": {                            // 항상 포함
    "base_path": "/home/user/blog",
    "content_paths": ["content/posts"],
    "image_path": "static/images",
    "hidden_path": ".hidden"
  }
}

// 사용자 응답 (prompt 이후, 필요한 만큼)
{
  "type": "prompt_response",
  "id": "<uuid>",                         // 보냈던 prompt의 id
  "value": "..."                          // confirm: bool, select: string|string[], input: string
}
```

### Plugin → Host 메시지

```json
// 진행률 (선택, 횟수 무제한)
{
  "type": "progress",
  "phase": "downloading",                 // 선택
  "current": 3, "total": 10,              // 선택 (수치 진행률)
  "message": "Fetching ..."               // 선택
}

// 사용자에게 묻기 (선택, 응답 받을 때까지 stdin readline에서 블록)
{
  "type": "prompt",
  "id": "<uuid>",                         // 응답 매칭용
  "kind": "confirm" | "select" | "input",
  "title": "...",
  "message": "...",                       // 선택
  "items": [                              // select 전용
    { "value": "v1", "label": "Display", "description": "..." }
  ],
  "multiple": true,                       // select 전용 (체크박스 vs 라디오)
  "default": "..."                        // input 전용 기본값
}

// 최종 결과 (반드시 1회, plugin 종료 직전)
{
  "type": "result",
  "success": true,
  "message": "Done",                      // 선택, toast로 표시
  "error": "...",                         // success=false 시 toast로 표시
  "actions": [ ... ]                      // 선택, 아래 PluginAction 참조
}
```

### PluginAction 타입

`result.actions[]`에 들어가는 후속 동작. 모두 호스트가 처리.

| Action | content | 효과 |
|---|---|---|
| `refresh_tree` | 없음 | 파일 트리 다시 로드 |
| `toast` | `{ message, toast_type: "success"\|"error"\|"info" }` | 알림 토스트 |
| `open_file` | `{ path }` | 에디터에서 파일 열기 |
| `show_result` | `{ title, body, pages? }` | 결과 팝업 (탭 + copy 블록 지원) |
| `download_files` | `{ items: [{path, filename, size}] }` | 서버 파일을 로컬로 다운로드 (체크박스 UI) |

`show_result.body`는 일반 텍스트. `{{copy:Title}}...{{/copy}}` 블록을 포함하면 접을 수 있는 복사 영역이 됨. `pages`로 여러 탭 구성 가능.

---

## Python 도우미 (참고용 스니펫)

호스트가 강제하는 라이브러리는 없음. 각 플러그인이 필요한 만큼 inline으로 작성. 표준 패턴:

```python
import json, sys, uuid

# (권장) 응답 전에 stdout이 flush되도록 line-buffered 모드
try: sys.stdout.reconfigure(line_buffering=True)
except (AttributeError, OSError): pass

def _send(msg):
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()

def send_progress(phase=None, current=None, total=None, message=None):
    msg = {"type": "progress"}
    if phase   is not None: msg["phase"]   = phase
    if current is not None: msg["current"] = float(current)
    if total   is not None: msg["total"]   = float(total)
    if message is not None: msg["message"] = message
    _send(msg)

def prompt_select(title, items, multiple=False, message=None):
    pid = str(uuid.uuid4())
    req = {"type": "prompt", "id": pid, "kind": "select",
           "title": title, "items": items, "multiple": multiple}
    if message: req["message"] = message
    _send(req)
    resp = json.loads(sys.stdin.readline() or "{}")
    if resp.get("type") != "prompt_response" or resp.get("id") != pid:
        return None
    return resp.get("value")

def send_result(success, message=None, error=None, actions=None):
    msg = {"type": "result", "success": success}
    if message: msg["message"] = message
    if error:   msg["error"]   = error
    if actions: msg["actions"] = actions
    _send(msg)

def main():
    data = json.loads(sys.stdin.readline())   # 첫 줄 = 초기 입력
    # ...
    send_result(True, message="OK", actions=[{"type": "refresh_tree"}])
```

**주의**:
- 첫 줄을 `sys.stdin.read()`로 읽으면 안 됨 — 호스트가 stdin을 계속 열어두기 때문에 EOF가 안 와서 영원히 블록됨.
- result는 반드시 한 번만, 마지막에 emit하고 종료.

---

## 호스트 아키텍처

### Rust

```
src-tauri/src/
├── types/plugin.rs           PluginManifest, Trigger, HookEvent, PluginResult,
│                             PluginAction, PluginProgress, PluginPrompt, PromptKind
├── services/plugin_service.rs
│   ├── discover_server_plugins()   서버의 plugin.json + .disabled + 해시
│   ├── compute_local_hash()        로컬 디렉토리 해시 (서버와 비교용)
│   ├── list_all_plugins()          로컬+서버 병합 → PluginInfo[]
│   ├── install_plugin()            로컬 → tar.gz → SFTP 업로드 → 서버에서 압축 해제
│   ├── pull_plugin()               역방향: 서버 → tar.gz → 로컬 해제
│   ├── enable/disable_plugin()     .disabled 마커 토글
│   ├── execute_plugin()            Manual 실행 — NDJSON 세션
│   ├── run_hooks()                 priority 정렬 후 hook 플러그인 순차 실행
│   ├── register/unregister_cron()  서버 crontab 관리
│   ├── respond_to_prompt()         프론트 응답 → 대기 중 plugin의 stdin으로 라우팅
│   └── run_ndjson_session()        SSH 채널로 NDJSON 메시지 루프
└── commands/plugin_command.rs      얇은 IPC 래퍼 (대부분 async)
```

`execute_plugin`은 `tauri::async_runtime::spawn_blocking`으로 돌림. plugin이 prompt에서 블록되는 동안 Tauri main thread는 자유로워야 다른 IPC(respond_to_plugin_prompt 포함)가 처리됨.

프롬프트 응답 라우팅:
- plugin → `prompt` 메시지 emit
- Rust: 전역 `Mutex<HashMap<id, mpsc::Sender>>`에 응답 채널 등록 + `plugin:prompt` Tauri 이벤트 emit
- frontend: 모달로 응답 수집 → `respond_to_plugin_prompt(id, value)` IPC 호출
- Rust IPC 핸들러: 레지스트리에서 sender 꺼내 send → execute_plugin 쓰레드 깨움 → stdin으로 응답 한 줄 write

### Svelte

```
src/sidebar/
├── PluginPanel.svelte           플러그인 목록 + install/enable/cron 토글.
│                                plugin:progress / plugin:prompt 리스너를 영구 보유.
│                                input이 0개인 trigger는 popup 없이 바로 invoke.
├── PluginInputPopup.svelte      input이 있는 trigger의 폼 + Execute 버튼.
├── PluginProgressModal.svelte   progress 메시지 → phase + 진행률 바.
├── PluginPromptModal.svelte     prompt 메시지 → confirm/select/input 모달.
├── PluginResultPopup.svelte     show_result 액션 → 탭 + copy 블록.
└── PluginDownloadPopup.svelte   download_files 액션 → 체크박스 + 진행률.
```

이벤트 라우팅:
- 리스너는 `PluginPanel` 마운트 시 한 번 등록 → 어디서 invoke했든 이벤트가 잡힘
- 프롬프트 응답은 `PluginPanel`이 IPC로 전송

---

## 현재 설치된 플러그인 (im-not-notion-plugins 레포)

| 이름 | 트리거 | 용도 |
|---|---|---|
| blog-backup | manual + cron(weekly) | Hugo 사이트 통째 tar.gz 백업 |
| git-autopush | manual + cron | 자동 commit + push |
| git-autosquash | manual | 기간 단위 커밋 squash |
| deploy-theme | manual | 테마 submodule 추가/업데이트 |
| remark42-setup | manual + cron | 댓글 서버 설치/백업 |
| goatcounter-setup | manual + cron | 방문자 분석기 설치/백업 |
| verify-image-link | manual | 이미지 참조 정합성 검증 |
| fix-image-link | manual | 깨진 이미지 참조 자동 복구 |
| wayback | manual | 외부 URL 단일 HTML 스냅샷 보관 (`monolith` 사용) |

---

## 실행 우선순위 (priority)

같은 이벤트/스케줄에 여러 플러그인이 등록됐을 때의 정렬 규칙:

- 필드: `trigger.priority` (정수, 기본 `50`)
- 정렬: 오름차순. 동일 시 플러그인 이름 알파벳순.

권장 범위:
- `1–19` 전처리
- `40–60` 일반 (기본값 50)
- `81–99` 검증/감사 (마지막에)

manual에는 적용되지 않음.

---

## 설치 / 동기화

PluginPanel UI의 동작:
- **로컬 path 지정** → `discover_local_plugins` 로 plugin.json 스캔
- **Install** (로컬 → 서버): 로컬 폴더를 tar.gz로 압축 후 SFTP 1회 업로드 → 서버에서 압축 해제 + CRLF 제거 + `chmod +x entry`
- **Pull** (서버 → 로컬): 서버에서 tar.gz 생성 → 다운로드 → 로컬 해제
- **Enabled toggle**: `.disabled` 마커 파일 생성/삭제
- **Synced 표시**: 로컬 해시와 서버 해시 비교 (각 파일 sha256 → 정렬 → 전체 sha256)
- **Cron toggle**: 트리거 단위로 crontab에 등록/해제. 비활성화 시 모든 cron 자동 해제.

설치 시 의존성 매니페스트는 없음 — 플러그인이 stdlib만 쓰거나, 서버에 미리 깔린 도구를 사용한다는 전제. wayback의 `monolith`처럼 외부 바이너리가 필요한 경우 플러그인이 직접 다운로드/설치 시도.

---

## 보안

- 플러그인 코드는 서버에서 SSH 유저 권한으로 실행 → 그 유저가 할 수 있는 모든 일을 할 수 있음
- base_path 외부 접근에 대한 샌드박싱 없음 — 신뢰할 수 있는 소스의 플러그인만 install
- prompt 응답 라우팅의 timeout은 600초

---

## 미결 사항

- [ ] hook 실패 시 정책 (현재: 에러 로그만, 무시)
- [ ] 플러그인 업데이트 알림 (서버/로컬 hash 불일치 표시는 있음, 자동 sync는 없음)
- [ ] 플러그인 stderr 캡처/표시 (현재 호스트가 안 읽음)
- [ ] 빌트인 플러그인 (앱 번들 포함) vs 외부 설치 구분
