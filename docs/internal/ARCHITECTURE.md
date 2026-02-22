# Architecture

## 1. Tech Stack

### Frontend
| Technology | Version | Role |
|-----------|---------|------|
| Svelte | 4.2.8 | Reactive UI framework |
| TypeScript | 5.0.2 | Type-safe JavaScript |
| Vite | 5.0.0 | Build tool / dev server (port 5573) |
| Tailwind CSS | 3.4.1 | Utility-first CSS |
| PostCSS | 8.4.35 | CSS processing |
| svelte-icons | 2.1.0 | Icon library |
| uuid | 9.0.1 | UUID generation |

### Backend
| Technology | Version | Role |
|-----------|---------|------|
| Rust | Edition 2021 | System language |
| Tauri | 2.x | Desktop framework (Webview wrapper) |
| ssh2 | 0.9.4 | SSH/SFTP client |
| serde + serde_json | 1.x | JSON serialization |
| aes-gcm | 0.10.3 | AES-256-GCM encryption |
| sha2 | 0.10.8 | SHA-256 hashing (key derivation) |
| base64 | 0.22.0 | Base64 encoding |
| dirs-next | 2 | Cross-platform home directory |
| indexmap | 2.10.0 | Ordered JSON maps |
| image | 0.25.0 | Image processing |
| once_cell | 1.19.0 | Lazy static initialization |
| anyhow | 1.0.80 | Error handling |
| ureq | 2.x | HTTP client (URL download) |
| typeshare | 1.x | Rust → TypeScript type generation |

---

## 2. Project Structure

### Top-Level Layout

```
im-not-notion/
├── src/                    # Frontend (Svelte + TypeScript)
├── src-tauri/              # Backend (Rust + Tauri 2.x)
├── public/                 # Static assets (SVGs)
├── plugins/                # Plugin source (local development)
├── docs/                   # Documentation
├── dist/                   # Vite build output
├── index.html              # Entry HTML
├── package.json            # Frontend dependencies & scripts
├── vite.config.ts          # Vite config (port 5573, base: '')
├── svelte.config.js        # Svelte preprocessor
├── tailwind.config.js      # Tailwind CSS
├── tsconfig.json           # TypeScript
└── postcss.config.js       # PostCSS (tailwind + autoprefixer)
```

### Frontend (`src/`)

```
src/
├── main.ts                 # App entry point
├── App.svelte              # Root component (layout: sidebar + topbar + content)
├── stores.ts               # Svelte writable stores (global state)
├── context.ts              # GLOBAL_FUNCTIONS context (symbol + interface)
├── theme.ts                # Theme toggle logic (system/light/dark, localStorage)
├── shortcut.ts             # Keyboard shortcut registry (registerAction/unregisterAction)
├── app.css                 # Global styles (Tailwind directives)
├── styles.css              # Theme CSS variables (light + dark mode)
│
├── fonts/
│   ├── D2Coding-Ver1.3.2-20180524.ttf
│   └── D2CodingBold-Ver1.3.2-20180524.ttf
│
├── types/
│   ├── generated.ts            # Auto-generated from Rust (#[typeshare])
│   ├── setting.ts              # Re-exports + createDefault* helpers
│   ├── svelte-icons.d.ts       # svelte-icons type declarations
│   ├── uuid.d.ts               # uuid type declarations
│   └── vite-env.d.ts           # Vite type declarations
│
├── sidebar/
│   ├── Sidebar.svelte          # Sidebar container + navigation
│   ├── Buttons.svelte          # Control buttons (settings, terminal, server, trash, reboot)
│   ├── FileControlSection.svelte  # File browser (search + refresh + tree)
│   ├── TreeNode.svelte         # Recursive file tree node (context menu: create/delete/rename)
│   ├── PluginPanel.svelte      # Plugin management UI
│   ├── HugoSetup.svelte        # Hugo setup wizard (New Site / Connect Existing)
│   ├── SettingsPopup.svelte    # Settings dialog (SSH tab + Hugo tab)
│   ├── PluginInputPopup.svelte # Manual plugin input form
│   ├── PluginResultPopup.svelte # ShowResult result display popup
│   ├── PluginDownloadPopup.svelte # Plugin download progress popup
│   ├── TerminalPopup.svelte    # SSH terminal emulator
│   └── RebootPopup.svelte      # Server reboot controls
│
├── content/
│   ├── MainContent.svelte      # Text editor + auto-save (5s interval)
│   └── SavePopup.svelte        # Save confirmation dialog
│
├── topbar/
│   └── TopBar.svelte           # File path display, browser open, hide/show toggle
│
├── component/
│   ├── Popup.svelte            # Reusable modal overlay
│   ├── DynamicField.svelte     # Dynamic form field renderer
│   └── Toast.svelte            # Toast notification (fly transition, auto-dismiss)
│
└── resource/
    ├── LogoSVG.svelte          # Logo SVG component
    ├── InvaderOpen.svelte      # Space invader open icon
    └── InvaderClose.svelte     # Space invader close icon
```

### Backend (`src-tauri/`)

```
src-tauri/
├── Cargo.toml              # Rust dependencies
├── tauri.conf.json         # Tauri app config (window 1024x768, bundle settings)
├── build.rs                # Tauri build script
├── icons/                  # App icons
│
└── src/
    ├── main.rs             # Entry: Tauri builder + IPC handler registration
    ├── lib.rs              # Library root (module declarations)
    │
    ├── commands/           # Tauri IPC command handlers (frontend에서 invoke)
    │   ├── mod.rs
    │   ├── config_command.rs   # load_config, save_config, switch_server, check_connection
    │   ├── file_command.rs     # get_file_tree, get/save_file_content, move, remove, toggle
    │   ├── ssh_command.rs      # start_server, kill_server, execute_ssh
    │   ├── setup_command.rs    # check_prerequisites ~ install_theme (10 commands)
    │   ├── pty_command.rs      # start/write/resize/stop PTY
    │   └── plugin_command.rs   # list/install/uninstall/enable/disable/run plugins + cron
    │
    ├── services/           # Business logic layer
    │   ├── mod.rs
    │   ├── config_service.rs   # Config load/save (local + remote SFTP)
    │   ├── ssh_service.rs      # SSH session singleton (Mutex), channel/SFTP operations
    │   ├── file_service.rs     # File tree, read/write, image sync, tree merge
    │   ├── setup_service.rs    # Hugo installation, site creation, theme install
    │   ├── pty_service.rs      # PTY session management over SSH
    │   └── plugin_service.rs   # Plugin discovery, execution, hooks, cron
    │
    ├── types/              # Data structures
    │   ├── mod.rs
    │   ├── plugin.rs           # PluginManifest, Trigger, HookEvent, PluginAction
    │   └── config/
    │       ├── mod.rs
    │       ├── app_config.rs       # AppConfig (frontend-facing unified config)
    │       ├── client_config.rs    # ClientConfig (local ~/.inn_config.json)
    │       ├── server_config.rs    # ServerConfig (remote ~/.inn_server_config.json)
    │       ├── server_entry.rs     # ServerEntry (multi-server management)
    │       ├── ssh_config.rs       # SshConfig (host, port, username, encrypted password)
    │       └── cms_config.rs       # CmsConfig > HugoConfig (paths, url, hidden_path)
    │
    └── utils/              # Utility modules
        ├── mod.rs
        ├── crypto.rs           # AES-256-GCM encryption (device UUID key derivation)
        └── error.rs            # IntoInvokeError trait
```

---

## 3. Architecture Overview

```
┌──────────────────────────────────────────────────────┐
│                    Tauri Desktop App                   │
│                                                       │
│  ┌─────────────────┐       ┌───────────────────────┐ │
│  │   Frontend       │       │   Backend (Rust)       │ │
│  │   (Webview)      │       │                        │ │
│  │                  │  IPC  │  commands/              │ │
│  │  Svelte + TS     │◄─────►│    ├ config_command    │ │
│  │  Tailwind CSS    │       │    ├ file_command      │ │
│  │                  │       │    ├ ssh_command        │ │
│  │  Stores:         │       │    ├ setup_command     │ │
│  │   relativeFile   │       │    ├ pty_command       │ │
│  │   isConnected    │       │    └ plugin_command    │ │
│  │   ...            │       │                        │ │
│  └─────────────────┘       │  services/             │ │
│                             │    ├ config_service    │ │
│                             │    ├ ssh_service       │ │
│                             │    ├ file_service      │ │
│                             │    ├ setup_service     │ │
│                             │    ├ pty_service       │ │
│                             │    └ plugin_service    │ │
│                             │                        │ │
│                             │  utils/                │ │
│                             │    ├ crypto            │ │
│                             │    └ error             │ │
│                             └──────────┬────────────┘ │
└──────────────────────────────────────┼────────────────┘
                                       │ SSH / SFTP
                              ┌────────▼────────┐
                              │  Remote Server   │
                              │  (Hugo Blog)     │
                              │                  │
                              │  ~/.inn_server_  │
                              │    config.json   │
                              │  ~/.inn_plugins/ │
                              │  /content/...    │
                              │  /static/images/ │
                              └─────────────────┘
```

---

## 4. Component Hierarchy

```
App
├── Sidebar
│   ├── Logo
│   ├── Buttons  ──> SettingsPopup, TerminalPopup, RebootPopup
│   │                 └── HugoSetup (inside SettingsPopup)
│   ├── FileControlSection
│   │   └── TreeNode (recursive)
│   ├── PluginPanel
│   │   ├── PluginInputPopup
│   │   ├── PluginResultPopup
│   │   └── PluginDownloadPopup
│   ├── Theme toggle button
│   └── Connection status indicator
├── TopBar
│   ├── File path display
│   ├── Hide/Show toggle
│   └── Open in browser
├── MainContent
│   ├── Textarea editor
│   └── SavePopup
└── Toast (global notification)
```

---

## 5. Data Flow

### 5.1 앱 시작 & 설정 로드
```
App mount
  → invoke("load_config")
  → config_service::load_config()
      → Read ~/.inn_config.json (local, SSH credentials)
      → Decrypt password (AES-256-GCM, device UUID key)
      → SSH connect to server
      → SFTP read ~/.inn_server_config.json (remote, Hugo config)
      → Return AppConfig to frontend
  → Stores update (url, contentPath, hiddenPath, isConnected)
```

### 5.2 파일 목록 조회
```
FileControlSection
  → invoke("get_file_tree")
  → file_service::build_file_tree()
      → SFTP stat content_paths (multiple sections)
      → Recursive directory listing (depth limit: 5)
      → SFTP stat hidden_path
      → Recursive directory listing (depth limit: 5)
      → merge_trees(public_tree, hidden_tree)
      → Return Vec<FileSystemNode>
  → TreeNode recursive rendering
```

### 5.3 파일 편집 & 저장
```
TreeNode click
  → relativeFilePath store update
  → MainContent reactive: invoke("get_file_content", relativeFilePath)
  → User edits textarea
  → Auto-save timer (5s) or manual save
  → invoke("save_file_content", relativeFilePath, content, manual)
  → manual=true: SFTP write + image sync + hooks
  → manual=false: SFTP write only
```

### 5.4 파일 숨기기/보이기 (Draft toggle)
```
TopBar hide/show toggle
  → invoke("toggle_hidden_file", path, hidden)
  → file_service: SFTP move file between content/ and hidden_content/
  → Refresh file list
```

---

## 6. State Management

### Frontend Stores (`stores.ts`)
```typescript
relativeFilePath   // 현재 선택된 파일의 상대 경로 (e.g., "posts/blog/hello.md")
selectedCursor     // 현재 활성 선택 (TreeNode highlight)
isConnected        // SSH 연결 상태
fullFilePath       // 전체 경로 (content_path 또는 hidden_path prefix 포함)
isEditingFileName  // 파일명 수정 모드 여부
draggingInfo       // Drag & Drop 상태 추적
```

### Backend Global State
```rust
// Lazy static singletons (Mutex-guarded)
APP_CONFIG: Lazy<Mutex<Option<AppConfig>>>   // In-memory config cache
SSH_CLIENT: Lazy<Mutex<Option<Session>>>     // Global SSH session
APP_HANDLE: OnceLock<AppHandle>              // Tauri app handle (for emit)
```

---

## 7. Configuration Files

| File | Location | Description |
|------|----------|-------------|
| `~/.inn_config.json` | Local machine | SSH credentials (password AES-256-GCM encrypted), multi-server entries |
| `~/.inn_server_config.json` | Remote server (via SFTP) | Hugo CMS config (paths, URL, hidden path), keyboard shortcuts |

### Configuration Split
- **ClientConfig** (local only): SSH credentials → encrypted password, server list
- **ServerConfig** (remote only): Hugo paths → no sensitive data, shortcuts
- **AppConfig**: unified frontend-facing struct combining both, marked with `#[typeshare]`

---

## 8. Security

### Password Encryption
- Algorithm: AES-256-GCM
- Key derivation: SHA-256(device UUID)
  - macOS: `IOPlatformUUID` via `ioreg`
  - Windows: `WMI` UUID via `wmic`
- IV: Random 12 bytes, stored as `iv:ciphertext` in base64
- Scope: Machine-locked (cannot decrypt on different device)

### Plugin Security
- 플러그인은 서버에서 유저 권한으로 실행 (SSH 접속 가능한 범위와 동일)
- JSON 프로토콜: stdin/stdout으로 데이터 교환
- base_path 외부 접근 가능 (제한 없음) → 유저 책임
