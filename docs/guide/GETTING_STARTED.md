# Getting Started

## 사전 요구사항

| Tool | 용도 | 설치 |
|------|------|------|
| **Rust** (stable) | Backend 빌드 | [rustup.rs](https://rustup.rs/) |
| **Node.js** (18+) | Frontend 빌드 | [nodejs.org](https://nodejs.org/) |

---

## 개발 환경 실행

### macOS / Linux

```bash
# 1. Rust 설치
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
. "$HOME/.cargo/env"

# 2. Node.js 설치 (https://nodejs.org/)

# 3. 의존성 설치 + 개발 서버 실행
npm install
npm run tauri dev
```

### Windows

```powershell
# 1. Rust 설치: https://rustup.rs/ 에서 rustup-init.exe 다운로드 후 실행
# 2. Node.js 설치: https://nodejs.org/ 에서 LTS 버전 설치

# 3. 의존성 설치 + 개발 서버 실행
npm install
npm run tauri dev
```

개발 서버는 Vite (port 5573) + Tauri 윈도우가 동시에 실행됩니다.

---

## 프로덕션 빌드

```bash
npm run tauri build
```

빌드 결과물:
- **macOS**: `src-tauri/target/release/bundle/dmg/`
- **Windows**: `src-tauri/target/release/bundle/msi/` 또는 `nsis/`
- **Linux**: `src-tauri/target/release/bundle/deb/` 또는 `appimage/`

---

## 설치 & 실행

### Windows

릴리즈 파일(`.msi` 또는 `.exe`)을 다운로드하여 설치하거나, 위 빌드 명령으로 직접 빌드.

### macOS

이 앱은 서명되지 않은 앱이므로 처음 실행 시 아래 명령이 필요합니다:

```bash
xattr -d com.apple.quarantine /path/to/im-not-notion.app
```

---

## 개발 스크립트

| 명령 | 설명 |
|------|------|
| `npm run tauri dev` | 개발 서버 (hot reload) |
| `npm run tauri build` | 프로덕션 빌드 |
| `npm run build` | Frontend만 빌드 (Vite) |
| `npm run typeshare` | Rust 타입 → TypeScript 자동 생성 (`src/types/generated.ts`) |
| `cargo check` | Backend 타입 체크 |

---

## 다음 단계

- 서버 초기 세팅: [SERVER_SETUP.md](./SERVER_SETUP.md)
- 전체 아키텍처: [../internal/ARCHITECTURE.md](../internal/ARCHITECTURE.md)
