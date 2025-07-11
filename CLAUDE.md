# im-not-notion 
Claude Code를 활용해 이 프로젝트를 효율적으로 개발 및 유지보수하기 위한 안내서입니다.
프로젝트명: im-not-notion
링크: https://github.com/parktest0325/im-not-notion

## 프로젝트 목적
`im-not-notion`은 docker 서버에서 실행되는 Hugo 기반 블로그의 Markdown 콘텐츠를 GUI로 작성·관리하기 위해 제작된 Tauri(Rust) + Svelte 데스크톱 애플리케이션입니다.

tauri 2.0을 사용하고 있다.


## 프로젝트 구조
| 경로                | 설명                                 |
| ----------------- | ---------------------------------- |
| `src/`            | Svelte 프론트엔드 소스코드                  |
| `src-tauri/`      | Rust 기반의 Tauri 백엔드 코드              |
| `public/`         | 정적 에셋 (SVG, 폰트 등)                  |


## 프론트엔드 규칙
 - 테마는 dark/light 테마를 지원하며 테마에 따른 여러 색상은 src/styles.css 에 정의되어 있다.
 - style을 관리하기 쉽도록 해야한다.
 - 코드는 최대한 깔끔하고 읽기 쉬워야하며, 각 요소가 너무 크지 않도록 atomic 하게 나누고 모듈화하여 재사용할 수 있도록 해야한다.

## 백엔드 규칙
 - mvc 패턴과 비슷하게 commands, services 으로 나뉘어 있다.
 - 코드는 최대한 깔끔하고 읽기 쉬워야한다.

## Git 커밋 규칙
- feat: 새로운 기능 추가
- fix: 버그 수정
- docs: 문서 수정
- style: 코드 포매팅 (기능 변경 없음)
- refactor: 코드 리팩토링 (기능 변경 없음)
- test: 테스트 추가 또는 수정
- chore: 빌드 프로세스 또는 보조 도구 변경

## 커밋 메시지 형식
<type>(<scope>): <subject>  (<issue_num>)

example.
feat(auth): 소셜 로그인 기능 추가   #27
fix(api): 사용자 조회 시 null 참조 오류 수정    #36

