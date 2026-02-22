# Plugin Feature

> im-not-notion 플러그인 시스템 설계 문서

**구현 상태:**
- [x] 플러그인 시스템 코어 (discover, execute, run_hooks, cron, install/uninstall)
- [x] 실행 우선순위 (priority 필드)
- [x] P1. Web Clipper — `plugins/web-clipper/`
- [x] P2. Git Auto-Push — `plugins/git-autopush/`
- [x] P3. Link Updater — `plugins/link-updater/`
- [x] P4. AI Draft — `plugins/ai-draft/`
- [x] P5. Verify — `plugins/verify/`

---

## 개요

서버 위 스크립트를 플러그인으로 실행하는 시스템.
기존 SSH 인프라를 재사용하며, 언어 무관 (Python, bash, Node.js 등).

핵심 원칙:
- **서버에 파일이 있으므로** 스크립트도 서버에서 실행 → SFTP 오버헤드 없음
- **기존 SSH 인프라 재사용** → 새로운 통신 채널 불필요
- **JSON 프로토콜** → stdin/stdout으로 앱과 데이터 교환
- **언어 무관** → shebang으로 인터프리터 지정

---

## 구현 대상 플러그인

### P1. Web Clipper — URL → 마크다운 변환

URL을 입력하면 페이지를 다운로드, 마크다운으로 변환, 지정 폴더에 저장.

```
사용자: URL 입력 + 대상 폴더 선택
  → 앱: SSH로 스크립트 실행
  → 스크립트: requests + html2text로 변환, 파일 저장
  → 앱: 파일 트리 새로고침
```

- 트리거: **Manual** (UI 버튼)
- 의존성: python3, requests, html2text (또는 beautifulsoup4)

### P2. Git Auto-Push — 블로그 자동 보전

10분마다 git push, 한 달 단위로 커밋 squash.

```
10분 주기 (cron):
  → cd {base_path} && git add -A && git commit -m "auto: $(date)" && git push

월간 (cron, 매월 1일):
  → 이전 달 커밋들을 squash하여 단일 커밋으로 병합
```

- 트리거: **Cron** (서버 crontab)
- 의존성: git, bash

### P3. Link Updater — 파일 이동 시 내부 링크 동기화

파일/폴더 이동 또는 이름 변경 시, 다른 글에서 참조하는 링크 경로를 자동 업데이트.

```
파일 이동 이벤트 발생
  → 앱: hook 스크립트에 src/dst 전달
  → 스크립트: base_path 내 모든 .md 파일에서 src 경로를 dst로 치환
  → 앱: 변경된 파일 목록 토스트 표시
```

- 트리거: **Hook** (AfterFileMove)
- 의존성: python3 (또는 bash sed)

### P4. AI Draft — AI 글 초안 생성

주제/키워드를 입력하면 LLM API로 블로그 글 초안을 생성하여 저장.
P1(Web Clipper)과 동일한 "입력 → 외부 소스 → 마크다운 저장" 패턴.

```
사용자: 주제 입력 + 대상 폴더 선택
  → 앱: SSH로 스크립트 실행
  → 스크립트: LLM API 호출 → frontmatter 포함 마크다운 생성 → 파일 저장
  → 앱: 파일 트리 새로고침
```

- 트리거: **Manual** (UI 버튼)
- 의존성: python3, openai (또는 anthropic 등 LLM SDK)
- 추가 필요: API 키 관리 (plugin.json `config` 필드 또는 서버 환경변수)

### P5. Verify — 데이터 정합성 검증

이미지 참조 정합성을 **검증**하는 복합 트리거 플러그인.
이미지 동기화 자체는 Rust 기본기능으로 처리되며, 이 플러그인은 검증/리포팅만 담당.

**Manual 트리거 (Verify Images):**
```
사용자: [Verify Images] 클릭 (입력 없음)
  → 전체 이미지 디렉토리 + md 파일 스캔
  → 전체 이미지 경로 목록 + Summary (Broken refs, Orphan files) 보고
  → 스냅샷을 .state.json에 저장 (baseline)
  → ShowResult 팝업으로 상세 보고서 표시
```

**Hook 트리거 (AfterFileSave/Move/Delete/Create):**
```
Rust 이미지 동기화 완료 후 hook 플러그인 실행
  → verify 플러그인 실행 (priority: 99, 항상 마지막)
  → .state.json baseline과 현재 상태 비교
  → 변경 감지: 추가/삭제/이동된 이미지 (UUID 기반 이동 감지)
  → baseline 업데이트
  → emit_hook_actions()로 Toast 전달
```

- 트리거: **Manual** + **Hook** (AfterFileSave, AfterFileMove, AfterFileDelete, AfterFileCreate)
- 우선순위: `99` (모든 hook 중 가장 나중에 실행)
- 상태 관리: `.state.json` (이미지 목록 스냅샷)
- 의존성: python3

---

## 실행 모드

| 모드 | 트리거 | 예시 | 앱 필요? |
|------|--------|------|----------|
| **Manual** | UI 버튼 클릭 | Web Clipper | O |
| **Hook** | 백엔드 함수 전/후 | Link Updater | O |
| **Cron** | 서버 crontab | Git Auto-Push | X |

---

## 실행 우선순위 (Priority)

같은 이벤트에 여러 Hook/Cron 플러그인이 등록된 경우, `priority` 값으로 실행 순서를 결정.

### 규칙

| 항목 | 값 |
|------|------|
| 필드 | `trigger.priority` (정수) |
| 기본값 | `50` (미지정 시) |
| 정렬 | **오름차순** — 낮은 숫자가 먼저 실행 |
| 동일 우선순위 | 플러그인 이름 알파벳순 (결정적 순서 보장) |

### 권장 범위

| 범위 | 용도 | 예시 |
|------|------|------|
| `1–19` | 전처리 (데이터 준비, 캐시 무효화) | — |
| `20–39` | 일반 초기 작업 | — |
| `40–60` | 기본 작업 (대부분의 플러그인) | Link Updater (`50`) |
| `61–80` | 후처리 (집계, 알림) | — |
| `81–99` | 검증/감사 (가장 나중에 실행) | Verify (`99`) |

### 적용 범위

- **Hook**: 동일 `event`에 등록된 플러그인 간 순서 결정
- **Cron**: 동일 `schedule`에 등록된 플러그인 간 순서 결정
- **Manual**: 해당 없음 (사용자가 직접 실행하므로)

### 실행 흐름 예시

```
AfterFileMove 이벤트 발생
  → priority 정렬: link-updater(50) → verify(99)
  → link-updater 실행 → 성공
  → verify 실행 → 정합성 검증 → 문제 발견 시 warning toast
```

### Rust 구현 — 구현 완료

> 파일: `src-tauri/src/services/plugin_service.rs`

`run_hooks()`는 매칭되는 hook을 먼저 수집한 뒤 priority 정렬 후 순차 실행:

```rust
pub fn run_hooks(event: HookEvent, data: Value) -> Result<Vec<PluginResult>> {
    let server_plugins = discover_server_plugins().unwrap_or_default();
    let hugo_config = get_hugo_config()?;

    // 매칭되는 hook 수집: (priority, plugin_name, entry)
    let mut matched: Vec<(u32, String, String)> = Vec::new();
    for (plugin, enabled, _) in &server_plugins {
        if !enabled { continue; }
        for trigger in &plugin.triggers {
            if let Trigger::Hook { event: e, priority } = trigger {
                if e == &event {
                    matched.push((
                        priority.unwrap_or(50),
                        plugin.name.clone(),
                        plugin.entry.clone(),
                    ));
                }
            }
        }
    }

    // priority 오름차순, 동일 시 이름순
    matched.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    // 정렬된 순서로 SSH 실행
    let mut results = Vec::new();
    for (_, name, entry) in &matched {
        // printf '%s' '{json}' | ~/.inn_plugins/{name}/{entry}
        // ...
    }
    Ok(results)
}
```

---

## 서버 디렉토리 구조

```
~/.inn_plugins/
├── web-clipper/
│   ├── plugin.json
│   ├── main.py
│   └── requirements.txt
├── git-autopush/
│   ├── plugin.json
│   └── main.sh
├── link-updater/
│   ├── plugin.json
│   └── main.py
├── ai-draft/
│   ├── plugin.json
│   ├── main.py
│   └── requirements.txt
└── verify/
    ├── plugin.json
    └── main.py
```

---

## plugin.json 스펙

> **중요**: Rust의 `Trigger` enum은 `#[serde(tag = "type", content = "content")]` (adjacently tagged).
> 따라서 trigger 필드는 `"type"` + `"content"` 구조를 따라야 한다.
> Manual의 `content`에는 `label`, `input` 등이, Hook의 `content`에는 `event`, `priority` 등이 들어간다.

```json
{
  "name": "web-clipper",
  "description": "Download URL and convert to markdown",
  "version": "1.0.0",
  "entry": "main.py",
  "triggers": [
    {
      "type": "manual",
      "content": {
        "label": "Clip URL",
        "input": [
          { "name": "url", "type": "string", "label": "URL" },
          { "name": "folder", "type": "string", "label": "Target folder", "default": "/clipped" }
        ]
      }
    }
  ]
}
```

```json
{
  "name": "git-autopush",
  "description": "Auto commit and push every 10 minutes",
  "version": "1.0.0",
  "entry": "main.sh",
  "triggers": [
    { "type": "cron", "content": { "schedule": "*/10 * * * *", "label": "Auto push" } },
    { "type": "cron", "content": { "schedule": "0 0 1 * *", "label": "Monthly squash" } }
  ]
}
```

```json
{
  "name": "link-updater",
  "description": "Update internal links when files are moved",
  "version": "1.0.0",
  "entry": "main.py",
  "triggers": [
    { "type": "hook", "content": { "event": "AfterFileMove" } }
  ]
}
```

```json
{
  "name": "verify",
  "description": "Verify data consistency — image reference integrity",
  "version": "2.0.0",
  "entry": "main.py",
  "triggers": [
    {
      "type": "manual",
      "content": {
        "label": "Verify Images",
        "input": []
      }
    },
    { "type": "hook", "content": { "event": "AfterFileSave", "priority": 99 } },
    { "type": "hook", "content": { "event": "AfterFileMove", "priority": 99 } },
    { "type": "hook", "content": { "event": "AfterFileDelete", "priority": 99 } },
    { "type": "hook", "content": { "event": "AfterFileCreate", "priority": 99 } }
  ]
}
```

```json
{
  "name": "ai-draft",
  "description": "Generate blog draft with AI",
  "version": "1.0.0",
  "entry": "main.py",
  "triggers": [
    {
      "type": "manual",
      "content": {
        "label": "AI Draft",
        "input": [
          { "name": "topic", "type": "string", "label": "Topic" },
          { "name": "folder", "type": "string", "label": "Target folder", "default": "/" }
        ]
      }
    }
  ],
  "config": {
    "api_key": { "type": "secret", "label": "API Key" },
    "model": { "type": "string", "label": "Model", "default": "gpt-4o" }
  }
}
```

---

## JSON 프로토콜

스크립트는 stdin으로 JSON을 받고, stdout으로 JSON을 반환.

### Manual 실행 (Web Clipper)

```json
// stdin (앱 → 스크립트)
{
  "trigger": "manual",
  "input": {
    "url": "https://example.com/article",
    "folder": "/clipped"
  },
  "context": {
    "base_path": "/home/user/mysite",
    "content_path": "posts",
    "image_path": "static/images"
  }
}

// stdout (스크립트 → 앱)
{
  "success": true,
  "message": "Saved to /clipped/example-article.md",
  "actions": [
    { "type": "refresh_tree" }
  ]
}
```

### Hook 실행 (Link Updater)

```json
// stdin (앱 → 스크립트)
{
  "trigger": "hook",
  "event": "AfterFileMove",
  "data": {
    "src": "/posts/old-name.md",
    "dst": "/posts/new-name.md"
  },
  "context": {
    "base_path": "/home/user/mysite",
    "content_path": "posts",
    "image_path": "static/images"
  }
}

// stdout (스크립트 → 앱)
{
  "success": true,
  "message": "Updated 3 files",
  "actions": [
    { "type": "toast", "content": { "message": "3 files updated", "toast_type": "success" } }
  ]
}
```

### 에러 시

```json
{
  "success": false,
  "error": "Failed to download URL: connection timeout"
}
```

---

## 아키텍처

### 백엔드 (Rust)

```
src-tauri/src/
├── types/
│   └── plugin.rs           # PluginManifest, Trigger, HookEvent 등
├── services/
│   └── plugin_service.rs   # 핵심 로직
│       ├── discover()           → 서버에서 플러그인 목록 조회
│       ├── install_deps()       → requirements.txt 기반 의존성 설치
│       ├── execute()            → SSH로 스크립트 실행 (stdin JSON → stdout JSON)
│       ├── run_hooks()          → 특정 이벤트에 등록된 hook 플러그인 실행
│       ├── register_cron()      → crontab에 스케줄 등록
│       └── unregister_cron()    → crontab에서 제거
├── commands/
│   └── plugin_command.rs    # IPC 커맨드 (thin wrapper)
│       ├── list_plugins()       → 프론트엔드에 플러그인 목록 전달
│       ├── run_plugin()         → Manual 플러그인 실행
│       ├── toggle_plugin()      → 활성화/비활성화
│       └── manage_cron()        → Cron 등록/해제
```

### 주요 타입

> 파일: `src-tauri/src/types/plugin.rs`

```rust
#[typeshare]
pub struct PluginManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub entry: String,
    pub triggers: Vec<Trigger>,
}

#[typeshare]
#[serde(tag = "type", content = "content")]  // adjacently tagged
pub enum Trigger {
    #[serde(rename = "manual")]
    Manual {
        label: String,
        input: Vec<InputField>,
        #[serde(default)]
        shortcut: Option<String>,
    },
    #[serde(rename = "hook")]
    Hook {
        event: HookEvent,
        #[serde(default)]
        priority: Option<u32>,   // 기본 50, 낮을수록 먼저 실행
    },
    #[serde(rename = "cron")]
    Cron {
        schedule: String,
        label: String,
        #[serde(default)]
        priority: Option<u32>,
    },
}

#[typeshare]
pub enum HookEvent {
    AfterFileMove,
    AfterFileSave,
    AfterFileDelete,
    AfterFileCreate,
}

#[typeshare]
pub struct InputField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub label: String,
    pub default: Option<String>,
}

/// 프론트엔드에 전달되는 플러그인 정보 (로컬+서버 병합)
#[typeshare]
pub struct PluginInfo {
    pub manifest: PluginManifest,
    pub local: bool,
    pub installed: bool,
    pub enabled: bool,
    pub synced: bool,
}

#[typeshare]
pub struct PluginResult {
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub actions: Vec<PluginAction>,
}

#[typeshare]
#[serde(tag = "type", content = "content")]
pub enum PluginAction {
    #[serde(rename = "refresh_tree")]
    RefreshTree,
    #[serde(rename = "toast")]
    Toast { message: String, toast_type: String },
    #[serde(rename = "open_file")]
    OpenFile { path: String },
    #[serde(rename = "show_result")]
    ShowResult { title: String, body: String },
}
```

### 핵심 함수: execute

```rust
/// 플러그인 스크립트를 SSH로 실행
pub fn execute(plugin_name: &str, input_json: &str) -> Result<PluginResult> {
    let mut channel = get_channel_session()?;

    // stdin으로 JSON 전달, stdout에서 JSON 수신
    let cmd = format!(
        "echo '{}' | ~/.inn_plugins/{}/{}",
        input_json, plugin_name, entry
    );
    let output = execute_ssh_command(&mut channel, &cmd)?;

    let result: PluginResult = serde_json::from_str(&output)?;
    Ok(result)
}
```

### Hook 통합

기존 서비스 함수에 이미지 동기화 + hook 호출을 삽입.
Hook 결과는 `emit_hook_actions()`로 프론트엔드에 전달.

- `move_content()`: copy-then-delete 트랜잭션 방식. 전체 md 참조 업데이트 후 hook 실행.
- `write_content(manual=true)`: 수동 저장 시 이미지 sync + hook 실행. sync 실패 시 warning toast.
- `write_content(manual=false)`: 자동 저장 시 순수 저장만. sync/hook 없음.
- `remove_content()`: 삭제 후 이미지 디렉토리 정리 + hook 실행.

상세 흐름은 `IMAGE_SYNC.md` 참조.

### Hook 결과 전달 — emit_hook_actions

> 파일: `src-tauri/src/main.rs`

`OnceLock<AppHandle>`에 저장된 글로벌 핸들을 통해 hook 결과의 actions를 프론트엔드로 emit:

```rust
pub fn emit_hook_actions(results: Vec<PluginResult>) {
    let Some(handle) = APP_HANDLE.get() else { return };
    for result in results {
        for action in result.actions {
            let _ = handle.emit("plugin-hook-action", &action);
        }
    }
}
```

프론트엔드 (`App.svelte`)에서 `listen("plugin-hook-action", ...)` 이벤트 리스너로 수신하여 toast/refresh_tree/show_result 처리.

### Cron 관리

```rust
/// crontab에 플러그인 스케줄 등록
pub fn register_cron(plugin_name: &str, schedule: &str, entry: &str) -> Result<()> {
    let marker = format!("# inn-plugin:{}", plugin_name);
    let job = format!(
        "{} cd ~/.inn_plugins/{} && ./{} {}",
        schedule, plugin_name, entry, marker
    );

    // 기존 항목 제거 후 추가
    let cmd = format!(
        "(crontab -l 2>/dev/null | grep -v 'inn-plugin:{}'; echo '{}') | crontab -",
        plugin_name, job
    );
    run_ssh(&cmd)?;
    Ok(())
}

/// crontab에서 플러그인 스케줄 제거
pub fn unregister_cron(plugin_name: &str) -> Result<()> {
    let cmd = format!(
        "crontab -l 2>/dev/null | grep -v 'inn-plugin:{}' | crontab -",
        plugin_name
    );
    run_ssh(&cmd)?;
    Ok(())
}
```

### 프론트엔드 (Svelte)

```
src/
├── sidebar/
│   ├── PluginPanel.svelte         # 플러그인 관리 UI
│   ├── PluginInputPopup.svelte    # Manual 플러그인 입력 폼
│   └── PluginResultPopup.svelte   # ShowResult 결과 표시 팝업
```

**PluginPanel 구성:**
```
┌─ Plugins ──────────────────┐
│                            │
│  ☑ web-clipper      [Run]  │
│    "Clip URL"              │
│                            │
│  ☑ git-autopush           │
│    ⏱ */10 * * * *  [On]   │
│    ⏱ 0 0 1 * *     [On]   │
│                            │
│  ☑ link-updater           │
│    🔗 AfterFileMove        │
│                            │
└────────────────────────────┘
```

- Manual: [Run] 버튼 → InputPopup → 실행 → 토스트/ShowResult 결과
- Cron: [On/Off] 토글 → crontab 등록/해제
- Hook: 활성/비활성 토글 (자동 실행, 수동 트리거 없음)

---

## 실행 흐름 정리

### Manual (Web Clipper)

```
User clicks [Run]
  → PluginInputPopup 표시 (url, folder 입력)
  → invoke("run_plugin", { name, input })
  → plugin_service::execute()
  → SSH: echo '{json}' | ~/.inn_plugins/web-clipper/main.py
  → stdout JSON 파싱
  → PluginAction 처리 (refresh_tree, toast 등)
```

### Hook (Link Updater + Verify)

```
User moves file (drag & drop or rename)
  → file_service::move_content() [copy-then-delete 트랜잭션]
  → Phase 1-3: 복사 + 이미지 복사 + 전체 md 참조 업데이트
  → Phase 4: 원본 삭제 (commit)
  → plugin_service::run_hooks(AfterFileMove, {src, dst})
  → priority 정렬: link-updater(50) → verify(99)
  → SSH: echo '{json}' | ~/.inn_plugins/link-updater/main.py
  → SSH: echo '{json}' | ~/.inn_plugins/verify/main.py → baseline 비교, 변경 감지
  → emit_hook_actions(results) → 프론트엔드에 toast/show_result 전달
```

### Manual (Verify Images)

```
User clicks [Verify Images]
  → invoke("run_plugin", { name: "verify", input: {} })
  → 전체 이미지/md 스캔 → baseline 저장 → 보고서 생성
  → PluginAction::ShowResult → PluginResultPopup에 상세 보고서 표시
  → 보고서: 전체 이미지 경로 목록 + Summary (Broken refs, Orphan files)
```

### Cron (Git Auto-Push)

```
User enables plugin in PluginPanel
  → invoke("manage_cron", { name, action: "register" })
  → plugin_service::register_cron()
  → SSH: crontab에 등록
  → 이후 서버에서 독립 실행 (앱 불필요)
```

---

## 플러그인 설치/배포

### 설치 방식

앱에서 플러그인 디렉토리를 SFTP로 업로드:

```
로컬 플러그인 zip/폴더 선택
  → SFTP로 ~/.inn_plugins/{name}/ 에 업로드
  → plugin.json 파싱하여 유효성 검증
  → dependencies.packages 자동 설치 (pip install -r requirements.txt)
  → 플러그인 목록 갱신
```

### 의존성 설치

```rust
pub fn install_deps(plugin_name: &str, runtime: &str, packages: &[String]) -> Result<()> {
    // runtime 존재 확인
    run_ssh(&format!("which {}", runtime))?;

    if !packages.is_empty() && runtime == "python3" {
        let req_path = format!("~/.inn_plugins/{}/requirements.txt", plugin_name);
        run_ssh(&format!("pip3 install --user -r {}", req_path))?;
    }
    Ok(())
}
```

---

## 보안 고려

- 플러그인은 서버에서 유저 권한으로 실행 → SSH 접속 가능한 범위와 동일
- 신뢰할 수 없는 소스의 플러그인 설치 시 경고 표시
- 플러그인이 base_path 외부 접근 가능 (제한 없음) → 유저 책임

---

## 미결 사항

- [x] ~~플러그인 간 실행 순서~~ → `priority` 필드로 구현 완료
- [x] ~~P5 Verify 플러그인~~ → `plugins/verify/` v2.0 구현 완료 (Manual + Hook + 상태 추적)
- [x] ~~ShowResult 액션~~ → `PluginAction::ShowResult` + `PluginResultPopup.svelte` 구현 완료
- [x] ~~hook 결과 프론트엔드 전달~~ → `emit_hook_actions()` + Tauri event 구현 완료
- [ ] hook 실패 시 정책 (현재: 무시. 향후: 롤백? 유저 선택?)
- [ ] 플러그인 업데이트 메커니즘
- [ ] 플러그인 로그 확인 UI
- [ ] 빌트인 플러그인 (앱에 기본 포함) vs 유저 설치 플러그인
