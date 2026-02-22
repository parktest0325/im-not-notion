# im-not-notion Documentation

> version: 1.1.0 | last updated: 2026-02-22

## Index

### Guide — 사용자 가이드

| Document | Description |
|----------|-------------|
| [GETTING_STARTED.md](./guide/GETTING_STARTED.md) | 빌드, 설치, 실행 방법 |
| [SERVER_SETUP.md](./guide/SERVER_SETUP.md) | 서버 초기 세팅 (SSH, 유저, Docker, 패키지) |
| [PLUGINS.md](./guide/PLUGINS.md) | 플러그인 사용 가이드 (autopush, autosquash, backup, image-link) |

### Internal — 내부 동작 가이드

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./internal/ARCHITECTURE.md) | Tech stack, 프로젝트 구조, 아키텍처, 데이터 흐름 |
| [IPC_API.md](./internal/IPC_API.md) | Frontend ↔ Backend IPC 커맨드 레퍼런스 (45개) |
| [IMAGE_SYNC.md](./internal/IMAGE_SYNC.md) | 이미지 관리: 저장/이동/붙여넣기 sync, 고아 정책, 플러그인 |
| [PLUGIN.md](./internal/PLUGIN.md) | 플러그인 시스템: manifest, trigger, JSON 프로토콜, 타입 |
| [MOVE_CONTENT_OPTIMIZATION.md](./internal/MOVE_CONTENT_OPTIMIZATION.md) | move_content SFTP 호출 최적화 계획 |

### Development

| Document | Description |
|----------|-------------|
| [TODO.md](./TODO.md) | 기능 추적, 리팩토링 노트 |
