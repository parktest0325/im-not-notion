# TODO / Improvement Notes

> 작업 진행 시 이 파일을 업데이트하며 추적합니다.
> TODO에는 유저가 직접 논의한 항목만 기록합니다.

---

## Feature

### [F1] Hugo Auto Setup — New Site / Connect Existing
- **상태**: 완료

SSH 설정 완료 후 Setup 화면에서 2가지 선택지 제공:

#### UI 위치
SettingsPopup > Hugo 탭 안에 Quick Setup 영역으로 배치
```
SettingsPopup
├── [SSH 탭]          ← SSH 설정 입력 (기존과 동일)
├── [Hugo 탭]
│   ├── Quick Setup 영역
│   │   ├── [New Site]           ← Hugo 설치 + 사이트 생성
│   │   └── [Connect Existing]   ← 기존 사이트 경로 입력
│   └── Config 필드들            ← base_path, content_path 등
│       (Quick Setup 완료 시 자동으로 채워짐, 수동 편집도 가능)
│
└── 하단 버튼: [Save] [Cancel]
```

#### 사용 전제조건
- 서버에 SSH 접속 가능한 계정만 있으면 됨
- sudo 불필요 (Hugo는 `~/.local/bin/`에 설치, 사이트는 `~/` 하위)
- `curl`, `tar`는 거의 모든 리눅스에 기본 설치됨
- `git`은 테마 설치 시 필요 → 없으면 경고만 표시, 진행은 가능

#### New Site 버튼
서버에 Hugo를 설치하고 새 사이트를 생성하여 im-not-notion에 연결
```
[New Site] 클릭
  → 사전 체크: curl, tar, git 존재 여부 확인 (git 없으면 경고만)
  → which hugo (설치 확인)
  → 없으면:
      uname -s, uname -m → OS/아키텍처 감지
      GitHub API로 최신 릴리즈 버전 조회
      curl -L로 바이너리 다운로드 + tar 압축 해제
      ~/.local/bin/hugo 에 배치
      hugo version 으로 설치 확인
  → 사이트 이름 생성 (그리스 알파벳 순차)
  → hugo new site ~/{name}
  → config 필드 자동 채움 (base_path, content_path, hugo_cmd_path 등)
  → 유저가 Save 클릭하면 저장
```

#### 사이트 이름 생성 규칙
그리스 알파벳 24개를 순서대로 순회하며, 한 사이클 끝나면 suffix 증가:
```
alpha → beta → gamma → ... → omega          (suffix 없음)
→ alpha-1 → beta-1 → gamma-1 → ... → omega-1  (suffix -1)
→ alpha-2 → beta-2 → gamma-2 → ... → omega-2  (suffix -2)
→ ...
```
서버 홈 디렉토리에서 첫 번째로 존재하지 않는 이름을 선택.

#### Connect Existing 버튼
기존 Hugo 프로젝트를 im-not-notion에 연결
```
SSH 연결 완료
  → 유저가 기존 Hugo 프로젝트 경로 입력 (e.g., ~/my-blog)
  → 경로 유효성 검증 (config.toml/hugo.toml 존재 확인)
  → Hugo 바이너리 설치 여부도 체크 (서버 재설치 등 대비)
  → config 자동 저장
  → 완료 → 메인 화면 전환
```

#### 결정된 사항
- [x] UI 위치: SettingsPopup > Hugo 탭 > Quick Setup 영역
- [x] 사이트 이름: 그리스 알파벳 순차 순회 + 사이클마다 suffix 증가
- [x] Hugo 설치: 공식 바이너리 직접 다운로드 (sudo 불필요, `~/.local/bin/`)
- [x] 사용 전제조건: SSH 접속 가능한 계정만 있으면 됨

#### 진행률 표시
Quick Setup 버튼 영역을 인라인 단계 목록으로 교체 (별도 창 없음):
```
[New Site] 클릭 후 (버튼 자리에):
┌─ Quick Setup ──────────────┐
│  ✓ curl, tar 확인           │
│  ✓ Hugo 설치 완료            │
│  ⏳ 사이트 생성 중...         │
│  ○ config 설정              │
└────────────────────────────┘
```
- 진행 중 Save/Cancel 비활성화
- 완료 시 아래 config 필드에 값 자동 채움

#### 미결 사항
(없음)

---

### [F2] SSH 세션 재사용 — 중복 연결 방지
- **상태**: 완료

현재 `connect_ssh`가 호출될 때마다 새 TCP 연결을 생성함.
`load_config`, `save_config` 등에서 반복 호출되어 짧은 시간에 다수의 SSH 연결이 발생.
Synology NAS 등에서 브루트포스 공격으로 오탐하여 IP 차단되는 문제.

#### 해결 방향
- 기존 세션이 살아있으면 `connect_ssh`를 건너뛰기
- `SSH_CLIENT`에 세션이 있고, 아직 유효한지 체크 후 재사용
- 세션이 끊어졌을 때만 재연결

---

### [F3] 토스트 알림
- **상태**: 완료

에러/성공 토스트 알림. 우측 하단에 3초간 표시 후 자동 소멸. 클릭 시 즉시 닫기.
- Toast store (`stores.ts`) + `addToast(message, type)` 헬퍼
- `Toast.svelte` 컴포넌트 (fly 트랜지션, error/success/info 색상)
- 에러 토스트 14건, 성공 토스트 9건 적용

---

### [F4] 코드 리팩토링
- **상태**: 완료 (S5 검색 미구현 제외)

---

#### 구조적 문제 (Architecture)

**[F4-A1] file_command thin wrapper화 — ✅ 완료**
- `file_command.rs` 230줄 → 52줄 (setup_command.rs와 동일 패턴)
- 비즈니스 로직(sftp_and_config, try_both, find_unique_path, path_exists 등) → `file_service.rs`로 이동
- 고수준 Hugo 함수 9개 추가: build_file_tree, read_content, write_content, write_image, create_content, remove_content, move_content, toggle_hidden, check_hidden
- `services/mod.rs` re-export 전부 제거, 직접 모듈 import로 변경

**[F4-A2] pty_service 역방향 의존 제거 — ✅ 완료**
- `pty_service`에서 `config_service::get_app_config()` 의존 제거
- `start_pty(cols, rows)` → `start_pty(host, port, username, password, cols, rows)` 파라미터 주입
- `pty_command`에서 config 추출 후 전달 (command → service 정방향)

**[F4-A3] get_server_home_path → ssh_service 이동 — ✅ 완료**
- SSH 유틸리티 함수를 config_service에서 ssh_service로 이동
- config_service import를 re-export 대신 직접 모듈 경로로 변경
- setup_service import 경로 변경

**[F4-A4] Frontend/Backend 타입 자동 동기화 (typeshare) — ✅ 완료**
- `typeshare` 크레이트 + CLI 도입, Rust 타입에 `#[typeshare]` 어트리뷰트 추가
- 대상: SshConfig, HugoConfig, CmsConfig, AppConfig, FileSystemNode, NodeType, PrerequisiteResult
- `src/types/generated.ts` 자동 생성, `setting.ts`는 re-export + createDefault* 헬퍼만 유지
- `serde(default)` 필드→구조체 레벨 이동 (typeshare optional 필드 문제 해결)
- FileSystemNode.children: `#[typeshare(serialized_as = "Vec<FileSystemNode>")]`로 IndexMap→Vec 매핑
- TreeNode.svelte: `NodeType` enum import, 문자열 비교 → enum 비교로 변경
- SettingsPopup.svelte: `asFields()` 캐스트 헬퍼 추가 (DynamicField Record 호환)
- `npm run typeshare`로 Rust 타입 변경 시 TypeScript 자동 재생성

**[F4-A5] GLOBAL_FUNCTIONS context 분리 — ✅ 완료**
- `src/context.ts` 신규 생성 (GLOBAL_FUNCTIONS symbol + GlobalFunctions interface)
- `stores.ts`에서 context 섹션 제거 (상태 + 토스트만 남음)
- App.svelte, TopBar.svelte, TreeNode.svelte, FileControlSection.svelte import 경로 변경

---

#### 코드 레벨 — Backend (Rust)

**[F4-R1] channel+execute 보일러플레이트 — ✅ 완료**
- `setup_service.rs`에 `run_ssh(cmd)` 헬퍼 추출, 20+ 반복 → 1줄 호출

**[F4-R2] sftp+hugo_config 보일러플레이트 — ✅ 완료**
- `sftp_and_config()` 헬퍼 추출, 9개 함수에서 2줄 → 1줄 (A1에서 file_service.rs로 이동)

**[F4-R3] dual-path 반복 패턴 — ✅ 완료**
- `try_both()` 제네릭 헬퍼 추출 (A1에서 file_service.rs로 이동)
- `remove_file` 16줄→4줄, `move_file_or_folder` 18줄→5줄

**[F4-R4] get_home_path 중복 — ✅ 완료**
- `get_server_home_path()`를 `pub`으로 변경 (A3에서 ssh_service.rs로 이동)
- `setup_service`에서 `echo $HOME` 3번 반복 제거, 기존 함수 재사용

**[F4-R5] kill_server 취약 — ✅ 완료**
- `ps/grep/awk` 파이프라인 → `pkill -f` 교체, 2채널→1채널

**[F4-R6] get_file_content/save_file_content 경로 — ✅ 완료**
- MainContent.svelte: `fullFilePath` → `relativeFilePath`로 변경 (invoke 호출 + 더블클릭 가드), `fullFilePath` import 제거
- file_service.rs: `/content/` 하드코딩 → `content_abs()` + `hidden_abs()` fallback (`or_else` 패턴)
- `save_file_image`과 동일한 `relativeFilePath` 기반 패턴으로 통일

**[F4-R7] get_file_list_() 이름 — ✅ 완료**
- `get_file_list_()` → `get_file_tree()` (backend + frontend 모두 변경)

**[F4-R8] depth 5 하드코딩 — ✅ 완료**
- `FILE_TREE_MAX_DEPTH` 상수 추출

---

#### 코드 레벨 — Frontend (Svelte)

**[F4-S1] console.log 잔류 — ✅ 완료**
- 10개 제거: TreeNode(7), FileControlSection(1), TopBar(1), RebootPopup(1)

**[F4-S2] commented-out 코드 — ✅ 완료**
- TreeNode: trashcan invoke 주석 제거
- TopBar: onMount 블록 10줄 제거, checkHidden guard 주석 제거

**[F4-S3] 미사용 변수 — ✅ 완료**
- TopBar: `let config: AppConfig`, `import { onMount }`, `import type { AppConfig }` 제거

**[F4-S4] class 바인딩 과다 — ✅ 완료**
- TopBar: 16개 `class:` 디렉티브 → `btn-visible`/`btn-hidden` CSS 클래스 2개 (`@apply`)

**[F4-S5] 검색 미구현 (LOW)**
- `FileControlSection.svelte` — searchFiles 빈 함수, UI는 존재

---

### [B1] 파일 이동/숨김 토글 시 대상 경로 중복 체크 누락
- **상태**: 완료

hidden 파일을 이동할 때 content 경로에 동일한 이름이 있어도 에러 없이 이동되는 버그.
`move_content`가 `try_both`로 content 실패 → hidden 시도하는데, hidden 쪽에는 중복이 없어서 성공.
`toggle_hidden`도 동일한 위험 (대상 경로에 이미 파일 존재 시 덮어쓸 수 있음).

- `move_content`: 이동 전 `path_exists()` (content+hidden 양쪽) 체크 추가
- `toggle_hidden`: 이동 전 `sftp.stat(dst)` 체크 추가

---

### [F5] 문서 작성
- **상태**: 완료

문서 구조 개편 완료:
- [x] docs/ 폴더 구조 재구성 (guide/ + internal/)
- [x] ARCHITECTURE.md + PROJECT_STRUCTURE.md 병합 → internal/ARCHITECTURE.md
- [x] IPC_API.md 전체 커맨드 업데이트 (45개)
- [x] GETTING_STARTED.md 신규 작성
- [x] README 인덱스 갱신

---

### [F6] Git Autosquash 테스트
- **상태**: 대기

`plugins/git-autosquash` 플러그인 실제 서버 환경에서 테스트.
- [ ] Manual trigger (since/until 날짜 지정) 테스트
- [ ] Cron trigger (지난달 자동 squash) 테스트
- [ ] 최신 커밋 이후에도 커밋이 있는 경우 (rebase 경로) 테스트
- [ ] force-with-lease push 테스트

---

### [F7] Hugo 커스텀 테마 제작
- **상태**: 대기

개인 블로그용 Hugo 테마 제작.

---

### [F8] AI 크롤링 플러그인
- **상태**: 대기

AI를 활용한 웹 크롤링 플러그인.

---

### [F9] AI 문서 작성 보조 플러그인
- **상태**: 대기

AI를 활용한 문서 작성 보조 플러그인.

---

### [F11] move_content SFTP 최적화
- **상태**: 대기

파일 이동 시 SFTP 왕복 15~18회 → 10~12회로 절감.
- [ ] Phase 3 + 3.5 통합 (같은 파일 read/write 중복 제거)
- [ ] `update_image_refs` 순수 함수 추출
- [ ] `move_file` parent 체크 파라미터화
- 상세: [MOVE_CONTENT_OPTIMIZATION.md](./internal/MOVE_CONTENT_OPTIMIZATION.md)

---

### [F10] 파일 업로드 액션
- **상태**: 대기

플러그인 액션에 `upload_files` 타입 추가 (클라이언트→서버).
- 파비콘/로고 등 Hugo 사이트 리소스 업로드
- 백업 복원 (restore)
- `download_files`의 반대 방향, 대칭 구조

---

## 작업 이력

| Date | ID | Action | Note |
|------|----|--------|------|
| 2026-02-19 | - | Initial docs creation | Project structure, architecture, API, TODO 문서화 |
| 2026-02-19 | F1 | Feature 논의 | Hugo Auto Setup (New Site / Connect Existing) 흐름 정리 |
| 2026-02-19 | F1 | UI/이름 확정 | Hugo 탭 내 Quick Setup 영역, 그리스 알파벳 순차 순회 이름 |
| 2026-02-19 | F1 | 설치/전제조건 확정 | 공식 바이너리 설치, SSH 계정만 있으면 됨, sudo 불필요 |
| 2026-02-19 | F1 | 진행률 확정 | 인라인 단계 목록, 별도 창 없음 |
| 2026-02-19 | F2 | 완료 | connect_ssh 세션 재사용, save 시만 reconnect_ssh 강제 재연결 |
| 2026-02-19 | F3 | 등록 | 에러 토스트 알림 (3초 후 자동 사라짐) |
| 2026-02-19 | F3 | 완료 | 에러 14건 + 성공 9건 토스트 적용, 영어 메시지 |
| 2026-02-19 | F4 | 상세 작성 | 구조 5건 (A1-A5) + 백엔드 8건 (R1-R8) + 프론트 5건 (S1-S5) |
| 2026-02-19 | F5 | 등록 | 플러그인 기능 (상세 미정) |
| 2026-02-19 | F4-S | 완료 | 프론트엔드 S1-S4 수정 (console.log 10개 제거, 주석코드 제거, 미사용 변수 제거, class 간소화) |
| 2026-02-19 | F4-R | 완료 | 백엔드 R1-R5,R7-R8 수정 (run_ssh 헬퍼, sftp_and_config 헬퍼, try_both 헬퍼, home_path 재사용, pkill, 함수명 변경, 상수 추출) |
| 2026-02-19 | F4-A | 완료 | 구조적 리팩토링 A1-A3,A5 (file_command thin wrapper화, pty_service 역방향 의존 제거, get_server_home_path ssh_service 이동, context.ts 분리) |
| 2026-02-19 | F4-A4 | 완료 | typeshare 도입 (Rust→TS 타입 자동 생성, generated.ts, serde(default) 구조체 레벨 이동, NodeType enum 적용) |
| 2026-02-19 | F4-R6 | 완료 | 파일 경로 통일 (fullFilePath→relativeFilePath, /content/ 하드코딩→content_abs()/hidden_abs()) |
| 2026-02-19 | B1 | 완료 | 파일 이동/숨김 토글 대상 경로 중복 체크 (move_content + toggle_hidden) |
| 2026-02-22 | - | 완료 | 이미지 관리 재구현 (IMAGE_SYNC.md 명세 기반, sync_images_on_save/move/paste 재작성) |
| 2026-02-22 | - | 완료 | 텍스트 붙여넣기 시 이미지 동기화 (sync_pasted_refs, preventDefault 방식) |
| 2026-02-22 | - | 완료 | verify/fix-image-link 플러그인 재작성 (v1.0.0, 5개 카테고리, collapsible 출력) |
| 2026-02-22 | - | 완료 | 플러그인 호환성 수정 (blog-backup, git-autosquash, git-autopush config 경로) |
| 2026-02-22 | F5 | 진행중 | 문서 구조 개편 (docs/tmp/ → docs/, README 인덱스, IMAGE_SYNC 보강) |
| 2026-02-22 | F6-F9 | 등록 | TODO 업데이트 (autosquash 테스트, hugo 테마, AI 플러그인 2종) |
