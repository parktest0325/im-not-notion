# IPC API Reference

Frontend에서 `@tauri-apps/api/core`의 `invoke()`로 호출하는 Backend command 목록.

---

## Config Commands (`config_command.rs`)

### `load_config`
- **Parameters**: none
- **Returns**: `AppConfig`
- **Description**: Local config (`~/.inn_config.json`) 로드 + SSH 접속 + Remote config (`~/.inn_server_config.json`) 로드
- **Side Effects**: SSH 연결 수립, `APP_CONFIG` 및 `SSH_CLIENT` global state 초기화

### `save_config`
- **Parameters**: `config: AppConfig`
- **Returns**: `Result<(), String>`
- **Description**: ClientConfig는 local에, ServerConfig는 SFTP로 remote에 저장
- **Side Effects**: 비밀번호 암호화 후 저장, SSH 재연결

### `save_plugin_local_path`
- **Parameters**: `path: String`
- **Returns**: `Result<(), String>`
- **Description**: 플러그인 로컬 경로를 ClientConfig에만 저장

### `check_connection`
- **Parameters**: none
- **Returns**: `bool`
- **Description**: SSH 연결 상태 확인

### `switch_server`
- **Parameters**: `servers: Vec<ServerEntry>`, `server_id: String`
- **Returns**: `AppConfig`
- **Description**: 활성 서버 전환 후 config 재로드

---

## File Commands (`file_command.rs`)

### `get_file_tree`
- **Parameters**: none
- **Returns**: `Vec<FileSystemNode>` (tree)
- **Description**: content_paths + hidden_path를 SFTP로 탐색 후 merge된 트리 반환
- **Note**: depth limit 5

### `get_file_content`
- **Parameters**: `file_path: String`
- **Returns**: `String`
- **Description**: SFTP로 파일 내용 읽기 (content/hidden 양쪽 탐색)

### `save_file_content`
- **Parameters**: `file_path: String`, `file_data: String`, `manual: bool`
- **Returns**: `Result<bool, String>`
- **Description**: SFTP로 파일 내용 쓰기
- **Note**: `manual=true` 시 이미지 sync + hook 실행, `manual=false` 시 순수 저장만

### `save_file_image`
- **Parameters**: `file_path: String`, `file_name: String`, `file_data: Vec<u8>`
- **Returns**: `String` (저장된 이미지 경로)
- **Description**: 이미지를 서버의 image_path에 업로드

### `new_content_for_hugo`
- **Parameters**: `file_path: String`
- **Returns**: `Result<String, String>`
- **Description**: Hugo frontmatter가 포함된 새 컨텐츠 파일 생성

### `move_file_or_folder`
- **Parameters**: `src: String`, `dst: String`
- **Returns**: `Result<(), String>`
- **Description**: 파일/폴더 이동 (SFTP rename + 이미지 이동 + 참조 업데이트)

### `remove_file`
- **Parameters**: `path: String`
- **Returns**: `Result<(), String>`
- **Description**: 파일/폴더 삭제 (recursive, 이미지는 고아로 유지)

### `toggle_hidden_file`
- **Parameters**: `path: String`, `state: bool`
- **Returns**: `Result<(), String>`
- **Description**: content ↔ hidden_content 간 파일 이동 (draft toggle)

### `check_file_hidden`
- **Parameters**: `path: String`
- **Returns**: `bool`
- **Description**: 파일이 hidden_path에 있는지 확인

### `download_remote_file`
- **Parameters**: `remote_path: String`, `local_path: String`
- **Returns**: `Result<(), String>`
- **Description**: 서버 파일을 로컬로 다운로드 (SFTP)

### `sync_pasted_refs`
- **Parameters**: `file_path: String`, `pasted_text: String`
- **Returns**: `String` (수정된 텍스트)
- **Description**: 붙여넣기 텍스트 내 외부 이미지 참조를 내 디렉토리로 복사 + 링크 수정

---

## SSH Commands (`ssh_command.rs`)

### `start_server`
- **Parameters**: none
- **Returns**: `Result<(), String>`
- **Description**: Hugo dev server 시작 (`nohup hugo server`)

### `kill_server`
- **Parameters**: none
- **Returns**: `Result<(), String>`
- **Description**: Hugo dev server 중지 (`pkill -f`)

### `execute_ssh`
- **Parameters**: `cmd: String`
- **Returns**: `String` (stdout + stderr)
- **Description**: 임의의 SSH 명령 실행 (Terminal popup에서 사용)

---

## Setup Commands (`setup_command.rs`)

### `check_prerequisites_cmd`
- **Parameters**: none
- **Returns**: `PrerequisiteResult`
- **Description**: 서버 사전 요구사항 확인 (curl, tar, git 존재 여부)

### `check_hugo_installed_cmd`
- **Parameters**: none
- **Returns**: `Option<String>`
- **Description**: Hugo 설치 여부 확인 (설치 시 버전 반환)

### `detect_server_platform_cmd`
- **Parameters**: none
- **Returns**: `(String, String)` — (OS, arch)
- **Description**: 서버 OS 및 아키텍처 감지 (`uname -s`, `uname -m`)

### `get_latest_hugo_version_cmd`
- **Parameters**: none
- **Returns**: `String`
- **Description**: GitHub API로 최신 Hugo 릴리즈 버전 조회

### `install_hugo_cmd`
- **Parameters**: `os: String`, `arch: String`, `version: String`
- **Returns**: `String` (설치 경로)
- **Description**: Hugo 바이너리를 서버에 설치 (`~/.local/bin/hugo`)

### `generate_site_name_cmd`
- **Parameters**: none
- **Returns**: `(String, String)` — (display name, slug)
- **Description**: 사이트 이름 생성 (그리스 알파벳 순차 순회)

### `create_hugo_site_cmd`
- **Parameters**: `hugo_cmd_path: String`, `site_path: String`
- **Returns**: `Result<(), String>`
- **Description**: 새 Hugo 사이트 생성

### `validate_hugo_project_cmd`
- **Parameters**: `path: String`
- **Returns**: `bool`
- **Description**: Hugo 프로젝트 유효성 검증 (config.toml/hugo.toml 존재 확인)

### `git_init_site_cmd`
- **Parameters**: `site_path: String`
- **Returns**: `Result<(), String>`
- **Description**: Hugo 사이트에 Git 저장소 초기화

### `install_theme_cmd`
- **Parameters**: `theme_url: String`, `site_path: String`
- **Returns**: `String` (테마 이름)
- **Description**: Git submodule로 Hugo 테마 설치

---

## PTY Commands (`pty_command.rs`)

### `start_pty_cmd`
- **Parameters**: `cols: u32`, `rows: u32`, `on_event: Channel<String>`
- **Returns**: `Result<(), String>`
- **Description**: SSH PTY 세션 시작 (streaming callback으로 출력 전달)

### `write_pty_cmd`
- **Parameters**: `data: String`
- **Returns**: `Result<(), String>`
- **Description**: PTY 세션에 데이터 쓰기

### `resize_pty_cmd`
- **Parameters**: `cols: u32`, `rows: u32`
- **Returns**: `Result<(), String>`
- **Description**: PTY 터미널 크기 변경

### `stop_pty_cmd`
- **Parameters**: none
- **Returns**: `Result<(), String>`
- **Description**: PTY 세션 종료

---

## Plugin Commands (`plugin_command.rs`)

### `list_plugins`
- **Parameters**: `local_path: String`
- **Returns**: `Vec<PluginInfo>`
- **Description**: 로컬 + 서버 플러그인 목록 조회 (병합)

### `install_plugin`
- **Parameters**: `local_path: String`, `name: String`
- **Returns**: `Result<(), String>`
- **Description**: 로컬 플러그인을 서버에 설치 (SFTP 업로드)

### `uninstall_plugin`
- **Parameters**: `name: String`
- **Returns**: `Result<(), String>`
- **Description**: 서버에서 플러그인 제거

### `enable_plugin`
- **Parameters**: `name: String`
- **Returns**: `Result<(), String>`
- **Description**: 플러그인 활성화 (`.disabled` 마커 제거)

### `disable_plugin`
- **Parameters**: `name: String`
- **Returns**: `Result<(), String>`
- **Description**: 플러그인 비활성화 (`.disabled` 마커 생성)

### `run_plugin`
- **Parameters**: `name: String`, `input: String`
- **Returns**: `PluginResult`
- **Description**: Manual 플러그인 실행 (SSH로 JSON stdin/stdout)

### `register_plugin_cron`
- **Parameters**: `name: String`, `schedule: String`, `entry: String`, `label: String`
- **Returns**: `Result<(), String>`
- **Description**: 플러그인 cron 스케줄 등록 (서버 crontab)

### `unregister_plugin_cron`
- **Parameters**: `name: String`, `label: String`
- **Returns**: `Result<(), String>`
- **Description**: 플러그인 cron 스케줄 해제

### `list_registered_crons`
- **Parameters**: none
- **Returns**: `Vec<String>`
- **Description**: 등록된 모든 cron 스케줄 조회

### `pull_plugin`
- **Parameters**: `local_path: String`, `name: String`
- **Returns**: `Result<(), String>`
- **Description**: 서버 플러그인을 로컬로 동기화

### `open_plugin_in_editor`
- **Parameters**: `local_path: String`, `name: String`
- **Returns**: `Result<(), String>`
- **Description**: 플러그인 디렉토리를 VS Code 또는 기본 에디터로 열기
