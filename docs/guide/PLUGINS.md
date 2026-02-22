# Plugin Guide

> im-not-notion 기본 제공 플러그인 사용 가이드

---

## 1. 공통 사항

### 설치 / 관리

플러그인은 앱 Sidebar의 Plugin Panel에서 관리합니다.

| 동작 | 방법 |
|------|------|
| 설치 | Plugin Panel > Install (로컬 → 서버로 SFTP 업로드) |
| 활성화/비활성화 | Plugin Panel > 토글 (`.disabled` 마커) |
| 제거 | Plugin Panel > Uninstall |
| Cron 등록 | Cron 트리거 옆 [On/Off] 토글 → 서버 crontab에 등록/해제 |

### 서버 디렉토리

```
~/.inn_plugins/
├── git-autopush/
│   ├── plugin.json
│   └── main.py
├── git-autosquash/
│   ├── plugin.json
│   └── main.py
├── blog-backup/
│   ├── plugin.json
│   └── main.py
├── verify-image-link/
│   ├── plugin.json
│   └── main.py
└── fix-image-link/
    ├── plugin.json
    └── main.py
```

### 사전 요구사항

| 플러그인 | 필요 패키지 |
|----------|------------|
| git-autopush | git |
| git-autosquash | git |
| blog-backup | tar |
| verify-image-link | python3 |
| fix-image-link | python3, requests (외부 URL 다운로드 시) |

---

## 2. git-autopush — 자동 커밋 & 푸시

블로그 변경 사항을 자동으로 git commit + push 합니다.

### 트리거

| 트리거 | 설명 |
|--------|------|
| **Manual** — "Git Commit & Push" | 즉시 커밋 + 푸시 |
| **Cron** — `*/10 * * * *` | 10분마다 자동 실행 |

### 동작

```
1. git status --porcelain → 변경 없으면 종료
2. git add -A
3. git commit -m "[날짜] auto" (cron) 또는 "[날짜] sync" (manual)
4. git push origin HEAD
```

### 에러 처리

- **SSH 키 미등록**: 서버에 SSH 키가 있으면 공개 키를 표시하고 Git 호스팅 등록 안내. 없으면 키 생성 가이드 표시.
- **Git user 미설정**: `git config --global user.name/email` 안내 표시.
- **Remote 미설정**: `git remote add origin` 안내 표시.

---

## 3. git-autosquash — 커밋 스쿼시

날짜 범위 내 커밋들을 하나로 합칩니다.

### 트리거

| 트리거 | 설명 |
|--------|------|
| **Manual** — "Squash Commits" | From/To 날짜 지정하여 스쿼시 |
| **Cron** — `0 0 1 * *` | 매월 1일 전월 커밋 자동 스쿼시 |

### Manual 사용법

| 입력 | 형식 | 설명 |
|------|------|------|
| From | `YYYY-MM-DD` | 시작 날짜 (포함) |
| To | `YYYY-MM-DD` | 종료 날짜 (**미포함**) |

날짜 범위는 **반개구간** `[From, To)` 입니다.

```
예: From=2026-01-01, To=2026-02-01
→ 1월 1일 00:00 ~ 1월 31일 23:59 커밋만 스쿼시
→ 2월 1일 커밋은 포함되지 않음
```

### 핵심 동작

1. **Author date 기준 필터**: `git log --since/--until`(committer date 기준)이 아닌, author date로 직접 필터합니다. rebase 후에도 정확한 날짜 필터가 보장됩니다.

2. **Squash 커밋 날짜**: 새로 만들어지는 squash 커밋의 author date는 범위 내 **마지막 커밋의 author date**를 상속합니다 (현재 시간이 아님).

3. **뒤따르는 커밋 보존**: 범위 이후의 커밋이 있으면 `git rebase --committer-date-is-author-date`로 날짜를 보존하면서 rebase합니다.

4. **Initial commit 처리**: 범위 내 첫 커밋이 repo 최초 커밋(parent 없음)이면, 해당 커밋은 유지하고 나머지만 squash합니다.

### 테스트 방법

squash 후 author date가 올바른지 확인:

```bash
# author date 기준으로 전체 로그 확인
git log --format="%h %aI %s"

# 오늘 날짜로 검색 — squash 커밋이 여기 나오면 잘못된 것
git log --format="%h %aI %s" | grep "2026-02-22"

# 정상: squash 커밋의 author date는 범위 내 마지막 커밋 날짜
# 예: [2026-01-01 ~ 2026-02-01] squash → author date가 1월 중
```

### 되돌리기 (Rollback)

squash를 되돌리려면 `git reflog`에서 squash 이전 상태를 찾아 복원합니다.

```bash
# 1. reflog에서 squash 이전 상태 찾기
git reflog | head -30

# 출력 예시:
# abc1234 HEAD@{0}: rebase (finish): returning to refs/heads/master
# ...
# def5678 HEAD@{8}: rebase (pick): [2026-02-16 13:20:01] Auto commit
# 137f26e HEAD@{9}: rebase (start): checkout inn_temp_squash   ← rebase 시작점
# e6c79a4 HEAD@{10}: checkout: moving from inn_temp_squash to master
# ...
# e6c79a4 HEAD@{12}: commit: [2026-02-22 15:20:01] auto       ← ★ 이것이 squash 직전 원래 상태

# 2. "rebase (start)" 바로 위의 "checkout: moving from inn_temp_squash to master" 항목을 찾기
#    그 해시가 squash 직전 원래 HEAD

# 3. 되돌리기
git reset --hard e6c79a4

# 4. 원격에도 반영 (force push 했었다면)
git push --force-with-lease origin HEAD
```

**주의**: reflog에서 `HEAD@{N}` 번호는 상황마다 다릅니다. 반드시 해시와 설명을 확인하세요.

---

## 4. blog-backup — 블로그 백업

Hugo 사이트를 tar.gz로 압축 백업합니다.

### 트리거

| 트리거 | 설명 |
|--------|------|
| **Manual** — "Backup Now" | 옵션 지정하여 즉시 백업 |
| **Cron** — `0 19 * * 0` | 매주 일요일 19:00 자동 백업 |

### Manual 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| Include .git directory | true | .git 폴더 포함 여부 |
| Include themes directory | true | themes 폴더 포함 여부 |
| Keep recent N backups | 5 | 최근 N개만 유지 (0 = 전부 유지) |

### 백업 위치

```
~/inn_backups/
├── blog_2026-02-22_190000.tar.gz
├── blog_2026-02-15_190000.tar.gz
└── ...
```

### 다운로드

백업 완료 후 `download_files` 액션이 반환되어, 앱에서 로컬로 다운로드할 수 있습니다.

---

## 5. verify-image-link — 이미지 링크 검증

전체 게시글과 이미지를 스캔하여 문제를 **리포트**합니다 (수정 없음).

### 트리거

| 트리거 | 설명 |
|--------|------|
| **Manual** — "Verify Image Links" | 전체 스캔 리포트 |

### 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| Verbose | true | 전체 이미지/파일 목록도 표시 |

### 감지 항목 (5개 카테고리)

| # | 항목 | 심각도 | 설명 |
|---|------|--------|------|
| 1 | 외부참조 | [X] | 다른 글의 이미지 참조 또는 외부 URL |
| 2 | 깨진 링크 | [X] | 링크는 있지만 이미지 파일 없음 |
| 3 | 대소문자 불일치 | [!] | 참조 경로와 실제 파일의 대소문자 차이 |
| 4 | 고아 이미지/폴더 | [!] | 어디서도 참조하지 않는 이미지 (section 내) |
| 5 | 중복 파일명 | [i] | 같은 이름이 여러 폴더에 존재 |

### Section / Non-Section 구분

- **Section 이미지**: `content_paths` 하위 이미지 → 고아 삭제 대상 (fix 플러그인)
- **Non-Section 이미지**: 파비콘, 로고 등 → `[i] Non-Section Files`로 별도 표시, 삭제 대상 아님

---

## 6. fix-image-link — 이미지 링크 일괄 수정

전체 게시글의 이미지 링크 문제를 **수정**합니다.

### 트리거

| 트리거 | 설명 |
|--------|------|
| **Manual** — "Fix Image Links" | 전체 스캔 + 수정 |

### 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| Dry run | false | 미리보기만 (실제 변경 없음) |
| Download external URLs | true | 외부 URL 이미지 다운로드 |
| Clean orphans | true | 고아 이미지/폴더 삭제 |

### 권장 워크플로우

```
1. verify-image-link 실행 → 문제 확인
2. fix-image-link (dry_run=true) → 변경 미리보기
3. fix-image-link (dry_run=false) → 실제 적용
4. verify-image-link 재실행 → 깨진 링크/대소문자 외 모두 해결 확인
```

### 수정 대상

| 문제 | 처리 |
|------|------|
| 외부참조 (다른 글 이미지) | 내 폴더로 복사 + 링크 수정 |
| 외부 URL (http/https) | 다운로드 + 내 폴더에 저장 + 링크 수정 |
| 고아 이미지 | 삭제 (section 내 이미지만) |
| 고아 폴더 | 삭제 (대응 md 없는 이미지 디렉토리) |

### fix로 해결되지 않는 항목

- **깨진 링크**: 원본 이미지가 어디에도 없음 → 수동 확인 필요
- **대소문자 불일치**: 어느 쪽이 맞는지 판단 필요 → 수동 수정
