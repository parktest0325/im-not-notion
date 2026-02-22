# 이미지 관리 기능 명세

> 구현 파일: `src-tauri/src/services/file_service.rs`

---

## 1. 이미지 기본 구조

### 1.1 저장 경로

이미지는 설정의 `image_path` 하위에 md 파일 경로를 그대로 따라 저장된다.
`hidden_path`, `content_path` 여부에 관계없이 동일한 이미지 경로를 공유한다.

```
저장 위치 (서버 절대경로):
  {base_path}/{image_path}/{section}/[폴더들...]/[파일명.md]/{UUID}

링크 주소 (md 내 참조, 항상 / prefix):
  /{section}/[폴더들...]/[파일명.md]/{UUID}

예시:
  저장: /home/user/mysite/static/images/posts/tech/article-a.md/a1b2c3
  링크: /posts/tech/article-a.md/a1b2c3
```

### 1.2 디렉토리 예시

```
Hugo 프로젝트 (base_path: /home/user/mysite)
├── content/
│   └── posts/                    ← content_path (section)
│       ├── tech/
│       │   ├── _index.md
│       │   └── article-a.md
│       └── life/
│           └── _index.md
├── content/.hidden/              ← hidden_path
│   └── posts/
│       └── tech/
│           └── draft.md          ← 숨김 파일
└── static/images/                ← image_path
    └── posts/                    ← section 포함
        ├── tech/
        │   ├── _index.md/
        │   │   └── a1b2c3        ← UUID 이미지
        │   └── article-a.md/
        │       ├── d4e5f6
        │       └── g7h8i9
        └── life/
            └── _index.md/
                └── j0k1l2
```

핵심 규칙:
- 각 md 파일은 자신의 이미지 디렉토리를 가짐: `{image_path}/{md파일의_상대경로}/`
- hidden/show 전환해도 이미지 위치는 변하지 않음 (같은 링크 공유)
- 여러 section이 있어도 이미지 경로에 section이 포함되어 충돌 없음

### 1.3 이미지 붙여넣기

```
에디터에서 이미지 붙여넣기:
  1. 클립보드에서 이미지 데이터 추출
  2. UUID 생성
  3. save_file_image(filePath, fileName=UUID) 호출
     → 서버 저장: {base_path}/{image_path}/{filePath}/{UUID}
     → 반환: {filePath}/{UUID}
  4. 에디터에 삽입: ![UUID](/{filePath}/{UUID})
```

링크 형태:
- **항상 앞에 `/`를 붙인다** (절대 경로 스타일)
- `![UUID이름](/posts/tech/article-a.md/a1b2c3)` 형태
- `posts/...`가 아닌 `/posts/...`로 통일

### 1.4 인식하는 이미지 참조 패턴

두 가지 패턴을 모두 인식하고 **동일하게** 처리한다:

| 패턴 | 예시 | 처리 |
|------|------|------|
| Markdown | `![alt](/posts/tech/a.md/uuid)` | 경로 파싱 + 복사/이동 + 치환 |
| HTML | `<img src="/posts/tech/a.md/uuid">` | 경로 파싱 + 복사/이동 + 치환 |
| 외부 URL (md) | `![alt](https://example.com/img.png)` | 다운로드 + 로컬 저장 + 치환 |
| 외부 URL (html) | `<img src="https://example.com/img.png">` | 다운로드 + 로컬 저장 + 치환 |

저장, 이동, 플러그인 모든 동작에서 두 패턴을 동일하게 처리한다.

---

## 2. 파일 저장 시 이미지 동기화

> 함수: `sync_images_on_save()` — `write_content(manual=true)` 에서만 실행

### 2.1 동작 요약

수동 저장 시 파일 내 모든 이미지 참조를 분석하여:
1. **외부참조** (다른 글의 이미지) → 내 디렉토리로 복사 + 링크 업데이트
2. **외부 URL** (`http(s)://`) → 다운로드 + 내 디렉토리에 저장 + 링크 업데이트
3. 저장 후 에디터에서 파일을 다시 읽어옴 (에디터에는 업데이트 전 내용이 있으므로)

### 2.2 외부참조 복사

```
상황:
  /posts/tech/article-a.md 에서 다른 글의 이미지를 참조:
  ![img](/posts/life/_index.md/j0k1l2)

동작:
  1. 참조 경로가 내 prefix (posts/tech/article-a.md/)로 시작하지 않음 → 외부참조
  2. 원본 이미지를 내 디렉토리로 복사:
     {image}/posts/life/_index.md/j0k1l2 → {image}/posts/tech/article-a.md/j0k1l2
  3. 링크 업데이트:
     ![img](/posts/life/_index.md/j0k1l2) → ![img](/posts/tech/article-a.md/j0k1l2)
  4. 수정된 md 내용을 서버에 저장

주의: 원본 이미지는 삭제하지 않는다 (다른 글에서 참조 중일 수 있음)
```

### 2.3 외부 URL 다운로드

```
상황:
  ![photo](https://example.com/image.png)

동작:
  1. URL에서 이미지 다운로드
  2. 파일명을 UUID로 생성하여 내 이미지 디렉토리에 저장
  3. 링크 업데이트:
     ![photo](https://example.com/image.png) → ![photo](/posts/tech/article-a.md/{UUID})
```

### 2.4 고아 이미지 정책

**저장 시 고아 이미지를 삭제하지 않는다.**

이유: 다른 글에서도 같은 이미지를 참조할 수 있기 때문에, 고아 상태로 두는 한이 있더라도 삭제하면 안 된다.

고아 이미지 정리는 `fix-image-link` 플러그인에서 수동으로 처리한다.

### 2.5 자동저장 제외

| | 수동 저장 (Ctrl+S / 저장 버튼) | 자동 저장 (주기적) |
|---|---|---|
| 내용 저장 | O | O |
| 이미지 sync | O | X |
| hook 실행 | O | X |
| 에디터 리로드 | sync 성공 시 | X |

자동저장에서 sync를 제외하는 이유:
- sync가 md 내 참조를 수정하면 에디터-서버 내용 불일치 (desync)
- 편집 중 이미지를 지웠다가 다시 넣을 수 있음
- 자동저장마다 hook 실행은 불필요하게 noisy

### 2.6 실행 흐름

```
write_content(file_path, data, manual=true):
  1. hidden/content 경로 판별 → 올바른 경로에 data 저장
  2. sync_images_on_save(file_path, data):
     a. 이미지 참조 파싱
     b. 외부참조 → 복사 + 링크 수정
     c. 외부 URL → 다운로드 + 저장 + 링크 수정
     d. 수정된 내용을 서버에 재저장
  3. run_hooks(AfterFileSave)
  4. 반환: Ok(true)

프론트엔드:
  - Ok(true): 서버에서 최신 파일 리로드 → 에디터 갱신 → "File saved."
  - Ok(false): 에디터 유지 → "File saved, but image sync failed."
  - Err: 에디터 유지 → "Failed to save file."
```

---

## 3. 텍스트 붙여넣기 시 이미지 동기화

> 함수: `sync_pasted_refs()` — 프론트엔드 `handlePaste()` 에서 호출

### 3.1 동작 요약

에디터에 텍스트를 붙여넣을 때 외부 이미지 참조가 포함되어 있으면:
1. 기본 붙여넣기를 **차단** (`preventDefault`)
2. 백엔드에서 이미지 복사 + 링크 수정 처리
3. **수정된 텍스트**를 에디터에 삽입

저장/리로드 없이 에디터 내에서 즉시 해결됨.

### 3.2 감지 대상

프론트엔드에서 붙여넣기 전에 아래 패턴을 검사:

| 패턴 | 예시 | 판정 |
|------|------|------|
| 다른 파일 이미지 | `![img](/posts/other.md/uuid)` | 외부참조 → sync |
| 외부 URL | `![img](https://example.com/img.png)` | 외부 URL → sync |
| 내 이미지 | `![img](/posts/mine.md/uuid)` | 정상 → 그대로 삽입 |
| 이미지 참조 없음 | `일반 텍스트` | 정상 → 그대로 삽입 |

### 3.3 실행 흐름

```
handlePaste(clipboardData):
  1. 이미지 데이터? → 기존 이미지 붙여넣기 로직 (UUID 생성 + 업로드)
  2. 텍스트에 외부 이미지 참조?
     → preventDefault()
     → invoke("sync_pasted_refs", filePath, pastedText)
       a. 외부참조 → 이미지 복사 + 링크 수정
       b. 외부 URL → 다운로드 + 저장 + 링크 수정
       c. 수정된 텍스트 반환
     → 에디터에 수정된 텍스트 삽입
     → "Image links synced." 토스트
  3. 일반 텍스트? → CodeMirror 기본 동작

실패 시:
  → 원본 텍스트 그대로 삽입 (수동 저장으로 재처리 가능)
```

### 3.4 저장 시 이중 처리

붙여넣기에서 이미 처리된 참조는 수동 저장 시 `sync_images_on_save`에서 다시 검사해도
이미 내 prefix로 시작하므로 스킵된다. 두 단계가 **충돌하지 않고 보완 관계**로 동작:

| 단계 | 역할 |
|------|------|
| 붙여넣기 (1차) | 즉시 처리 — 외부참조가 에디터에 남지 않음 |
| 수동 저장 (2차) | 잔여 처리 — 직접 타이핑한 참조, 레거시 데이터 |

---

## 4. 파일/폴더 이동 시 이미지 동기화

> 함수: `move_content()`, `sync_images_on_move()`

### 4.1 동작 요약

파일/폴더가 이동되거나 이름이 변경되면:
1. 이미지 폴더를 통째로 새 경로로 이동
2. 이동된 파일 내 이미지 링크를 새 경로로 업데이트
3. 외부참조 (다른 파일 참조)도 파일 저장과 동일한 방법으로 복사 + 업데이트
4. 폴더 이동 시 하위 파일 전체에 일괄 적용

### 4.2 트랜잭션 흐름

```
move_content(src, dst):
  Phase 1: content/hidden 파일을 dst로 이동 (rename)
    → 실패 시: 에러 반환

  Phase 2: 이미지 디렉토리를 dst로 이동 (rename)
    → 실패 시: Phase 1 롤백 → 에러 반환

  Phase 3: 이미지 참조 업데이트
    a. 이동된 파일 내 자기참조: old_prefix → new_prefix
    b. 외부참조: 다른 글의 이미지 → 내 디렉토리로 복사 + 링크 수정
    → 실패 시: Phase 1,2 롤백 → 에러 반환

  Hook 실행
```

### 4.3 참조 업데이트 범위

이동 시 참조 업데이트 대상:

| 대상 | 방법 |
|------|------|
| 이동된 파일 자체 (단일 파일) | old_prefix → new_prefix 치환 |
| 이동된 폴더 내 모든 md 파일 | 하위 파일 전체에 old_prefix → new_prefix 치환 |

```
예시: article-a.md를 /posts/tech/ → /posts/dev/로 이동

article-a.md 내부:
  ![img](/posts/tech/article-a.md/uuid) → ![img](/posts/dev/article-a.md/uuid)
```

### 4.4 Hidden 토글

hidden으로 변경되는 경우 글의 실제 경로(content → hidden 또는 반대)는 이동되지만,
이미지는 동일한 경로(`{image_path}/{section}/...`)를 사용하므로 이미지 이동이 필요 없다.

단, 파일 내에 외부참조가 있다면 저장 시(2절)의 로직과 동일하게 처리한다.

### 4.5 롤백 보장

| 실패 단계 | 결과 |
|-----------|------|
| Phase 1 (content 이동) | 에러 반환 |
| Phase 2 (이미지 이동) | Phase 1 롤백, 원본 유지 |
| Phase 3 (참조 업데이트) | Phase 1,2 롤백, 원본 유지 |

---

## 5. 에러 처리

### 5.1 중복 이미지 (동일 파일명 존재)

이미지를 복사할 때 대상 경로에 이미 동일한 파일명이 존재하는 경우:

```
동작:
  1. 원본과 대상의 파일 해시(SHA-256)를 비교
  2. 동일한 파일이면: 복사 스킵, 링크만 업데이트
  3. 다른 파일이면: 새로운 UUID 이름으로 복사, 링크를 새 이름으로 업데이트
```

이 방식으로 한 파일 내에서 같은 이미지를 중복 참조해도 용량이 늘어나지 않는다.

### 5.2 네트워크/SFTP 실패

```
파일 저장 시:
  - 이미지 복사 실패 → 해당 이미지 스킵, 다른 이미지는 계속 처리
  - 전체 sync 실패 → 원본 내용(링크 미수정)은 이미 저장됨
    → Ok(false) 반환 → "File saved, but image sync failed." 토스트

파일 이동 시:
  - 컨텐츠 파일은 이동됐지만 이미지 파일이 이동되지 않은 경우
    → 링크도 업데이트하지 않고 컨텐츠 파일을 이전 위치로 롤백
  - 링크 업데이트에서 실패한 경우
    → 이미지 파일과 컨텐츠 파일 모두 이전 위치로 롤백
```

### 5.3 외부 URL 다운로드 실패

```
동작:
  1. URL에서 다운로드 실패 → 해당 링크 스킵 (원본 URL 유지)
  2. 다른 이미지는 계속 처리
  3. 실패한 항목이 있어도 성공한 항목은 반영
```

---

## 6. 파일 삭제

> 함수: `remove_content()`

파일/폴더 삭제 시 이미지 디렉토리는 **삭제하지 않고 고아로 둔다.**

```
동작:
  1. content/hidden 양쪽에서 파일/폴더 삭제
  2. 이미지 디렉토리는 그대로 유지 (고아 상태)
  3. Hook 실행

이유: 다른 파일에서 외부참조하고 있을 수 있으므로 함부로 삭제하면 안 됨.
정리: verify-image-link 플러그인이 "Orphan Image Dir"로 감지 → fix-image-link으로 수동 정리
```

---

## 7. 플러그인: fix-image-link (일괄 수정)

> 서버에서 직접 실행. 수동 트리거(manual). 내장 sync와 별개로 전체 게시글을 일괄 처리.

### 7.1 동작

전체 게시글(content + hidden)을 스캔하여 이미지 링크 문제를 수정한다.

```
1. 전체 게시글 스캔 → 모든 이미지 참조 수집
2. 외부참조/외부URL 처리:
   - 다른 글의 이미지를 참조 → 내 폴더로 복사 + 링크 수정
   - 외부 URL (http(s)://) → 다운로드 + 내 폴더에 저장 + 링크 수정
   - ![](...)와 <img src="..."> 모두 처리
3. 고아 정리 (전체 기준):
   - 어떤 게시글에서도 참조하지 않는 이미지 파일 삭제
   - 대응하는 md가 없는 이미지 폴더 삭제
```

### 7.2 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `dry_run` | false | 미리보기만, 실제 변경 없음 |
| `download_urls` | true | 외부 URL 다운로드 여부 |
| `clean_orphans` | true | 고아 이미지/폴더 삭제 여부 |

### 7.3 고아 삭제 범위

내장 sync(파일 저장 시)에서는 고아를 삭제하지 않는다.
fix-image-link 플러그인에서만 **전체 폴더 기준**으로 고아를 삭제한다.

```
전체 이미지 - 전체 게시글에서 참조하는 이미지 = 고아

고아 이미지: 어떤 글에서도 참조하지 않는 이미지 파일 → 삭제
고아 폴더: 대응하는 md 파일이 없는 이미지 디렉토리 → 삭제
```

---

## 8. 플러그인: verify-image-link (검증 리포트)

> 서버에서 직접 실행. 수동 트리거(manual). 수정 없이 리포트만 출력.

### 8.1 감지 항목

전체 게시글(content + hidden)과 이미지를 스캔하여 문제를 리포트한다.

| # | 감지 항목 | 심각도 | 설명 | fix 연관 |
|---|----------|--------|------|----------|
| 1 | **외부참조** | [X] | 다른 글의 이미지 또는 외부 URL 참조 | fix 6.1-2로 해결 |
| 2 | **깨진 링크** | [X] | 링크는 있지만 실제 이미지 파일이 없음 | — |
| 3 | **대소문자 불일치** | [!] | 참조 경로와 실제 파일의 대소문자가 다름 (Linux에서 깨짐) | — |
| 4 | **고아 이미지/폴더** | [!] | 어떤 게시글에서도 참조하지 않는 이미지 또는 md 없는 폴더 | fix 6.1-3으로 해결 |
| 5 | **중복 파일명** | [i] | 같은 이미지 이름이 여러 폴더에 존재 (fix에서 복사된 파일) | — |

### 8.2 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `verbose` | true | 전체 이미지/파일 목록도 출력 |

### 8.3 fix와의 관계

```
verify 실행 → 문제 리포트 확인 → fix 실행 (dry_run으로 미리보기) → fix 실행 (적용)
→ verify 재실행 → 깨진 링크/대소문자 외에는 모두 해결되어야 함

verify에서만 감지되고 fix로 해결되지 않는 항목:
  - 깨진 링크 (원본 이미지가 어디에도 없음 → 수동 확인 필요)
  - 대소문자 불일치 (어느 쪽이 맞는지 판단 필요 → 수동 수정)
```

---

---

## 9. 테스트 시나리오

### 9.1 수동 저장 (Ctrl+S)

| # | 시나리오 | 기대 결과 |
|---|---------|----------|
| S1 | 자기 이미지만 참조 | 변경 없음, 링크 유지 |
| S2 | 다른 글 이미지 참조 (외부참조) | 내 폴더로 복사 + 링크 `/내경로/파일명` 으로 변경 |
| S3 | 외부 URL 이미지 | 다운로드 + 내 폴더에 저장 + 링크 변경 |
| S4 | 동일 파일명 충돌 (같은 내용) | 복사 스킵 + 링크만 변경 |
| S5 | 동일 파일명 충돌 (다른 내용) | 새 UUID 이름으로 복사 + 링크 새 이름으로 변경 |
| S6 | 깨진 참조 (원본 없음) | 스킵 (링크 유지) |
| S7 | Hidden 파일 저장 | 이미지는 동일 경로, hidden_path 미포함 |
| S8 | 자동저장 | sync 실행 안함 (내용만 저장) |

### 9.2 파일/폴더 이동

| # | 시나리오 | 기대 결과 |
|---|---------|----------|
| M1 | 파일 이름 변경 | 이미지 폴더 rename + 내부 링크 업데이트 |
| M2 | 파일 다른 폴더로 이동 | 이미지 폴더 이동 + 내부 링크 업데이트 |
| M3 | 폴더 이름 변경 | 하위 모든 md의 이미지 폴더 + 링크 일괄 업데이트 |
| M4 | 폴더 채로 다른 위치로 이동 | 하위 전체 이미지 + 링크 일괄 업데이트 |
| M5 | 이동 후 외부참조 존재 | Phase 3.5에서 외부참조 복사 + 링크 수정 |
| M6 | Hidden 파일의 폴더 이동 | hidden_base 먼저 체크 → 이미지 경로에 hidden_path 미포함 |
| M7 | Legacy 이미지 경로 (섹션 없음) | legacy → 새 형식으로 변환 + 링크 업데이트 |

### 9.3 파일 삭제

| # | 시나리오 | 기대 결과 |
|---|---------|----------|
| D1 | 파일 삭제 | content/hidden 삭제, 이미지는 고아로 유지 |
| D2 | 폴더 삭제 | 폴더 전체 삭제, 이미지는 고아로 유지 |

### 9.4 플러그인

| # | 시나리오 | 기대 결과 |
|---|---------|----------|
| P1 | verify → 정상 상태 | "All image links are consistent!" |
| P2 | verify → 문제 있음 | 카테고리별 카운트 표시 (verbose=false) / 상세 경로 (verbose=true) |
| P3 | fix (dry_run) | 변경 없이 미리보기만 |
| P4 | fix (적용) → verify | 외부참조/고아 해결됨, 깨진 링크/대소문자만 남음 |
| P5 | fix → 외부 URL 다운로드 실패 | 해당 URL 스킵, 나머지 정상 처리 |

---

## 10. 링크가 깨지는 경우 (Edge Cases)

### 10.1 이동으로 인한 외부 파일 깨짐 (Known Limitation)

```
상황:
  파일 A가 파일 B의 이미지를 참조: ![img](/posts/b.md/uuid)
  파일 B를 /posts/c.md로 이동

결과:
  B의 이미지 디렉토리: posts/b.md/ → posts/c.md/ 으로 이동됨
  A의 링크 /posts/b.md/uuid → 이미지가 없어짐 (깨진 링크)

이유:
  이동 시 "이동된 파일 내부"의 참조만 업데이트.
  "다른 파일이 이동된 파일의 이미지를 참조"하는 경우는 업데이트하지 않음.

해결:
  1. A를 수동 저장 → sync가 외부참조 감지 → 새 경로에서 복사 시도
     → 하지만 원본 경로(posts/b.md/)가 이미 없으므로 복사 실패 → 깨진 링크 유지
  2. verify-image-link으로 감지 → 수동 수정 필요
```

**이것이 유일하게 자동 복구 불가능한 케이스다.**

### 10.2 SFTP 연결 끊김 (Partial Sync)

```
상황:
  sync_images_on_save 중 3개 이미지 중 2번째에서 SFTP 실패

결과:
  1번째 이미지: 복사 완료 + 링크 수정됨
  2번째 이미지: 복사 실패 → 링크 유지 (원본 경로)
  3번째 이미지: 처리 안됨
  수정된 내용(1번만 반영)이 서버에 저장됨

복구:
  다시 수동 저장하면 2,3번째가 다시 sync됨
```

### 10.3 이동 중 Phase 2 실패 (이미지 이동 실패)

```
상황:
  Phase 1(content 이동) 성공 → Phase 2(이미지 이동) 실패

결과:
  Phase 1 롤백 → 원본 상태 유지
  링크 영향 없음 (깨지지 않음)
```

### 10.4 이동 중 Phase 3 실패 (참조 업데이트 실패)

```
상황:
  Phase 1, 2 성공 → Phase 3(참조 업데이트) 실패

결과:
  Phase 1, 2 롤백 → 원본 상태 유지
  링크 영향 없음 (깨지지 않음)

주의: Phase 3.5(외부참조 sync)는 실패해도 롤백하지 않음
  → 복사된 이미지가 고아로 남을 수 있지만 기존 링크는 깨지지 않음
```

### 10.5 동일 URL 내용 변경

```
상황:
  ![img](https://example.com/photo.png)를 저장하여 다운로드됨
  나중에 URL의 이미지가 변경됨
  다시 저장

결과:
  URL 해시로 파일명 생성 → 동일 파일명
  이미 다운로드된 파일이 있으므로 스킵 (이전 버전 유지)
  → 의도적 동작: 한번 다운로드 후에는 로컬 복사본 사용
```

### 10.6 Markdown title 문법

```
상황:
  ![alt](/path/uuid "이미지 설명") — Markdown title 구문

결과:
  경로를 `/path/uuid "이미지 설명"`으로 파싱
  파일 찾기 실패 → 스킵 (깨지지는 않지만 sync 안됨)

참고: 앱의 에디터는 이 형식을 생성하지 않으므로 실질적 영향 없음
```

### 10.7 대소문자 불일치 (Linux 서버)

```
상황:
  이미지 파일: posts/tech/Article-A.md/uuid
  링크: /posts/tech/article-a.md/uuid (소문자)

결과:
  macOS: 동작함 (대소문자 무시)
  Linux: 404 (깨진 링크)

감지: verify-image-link의 "Case Mismatches" 카테고리
해결: 수동 수정 필요
```

---

## 11. 내장 sync vs 플러그인 비교

### 11.1 역할 분담

| | 내장 (Rust, SFTP) | 플러그인 (Python, 서버 로컬) |
|---|---|---|
| **실행 위치** | 클라이언트 → SFTP로 서버 조작 | 서버에서 직접 실행 |
| **트리거** | 자동 (저장/이동/붙여넣기) | 수동 (Manual trigger) |
| **범위** | 현재 파일 1개 | 전체 게시글 일괄 |
| **외부참조 복사** | O | O |
| **외부 URL 다운로드** | O | O |
| **이동 시 링크 업데이트** | O | X (이동 기능 없음) |
| **고아 이미지 삭제** | X (안전상 삭제 안함) | O (section 내 한정) |
| **고아 폴더 삭제** | X | O (section 내 한정) |
| **검증 리포트** | X | O (verify 플러그인) |
| **Non-Section 파일 감지** | X | O (verify에서 [i]로 표시) |

### 11.2 처리 흐름 비교

```
내장 sync (파일 단위, 실시간):
  붙여넣기 → sync_pasted_refs() → 즉시 처리
  수동 저장 → sync_images_on_save() → 잔여 처리
  파일 이동 → sync_images_on_move() → 링크 업데이트
  파일 삭제 → 이미지 유지 (고아로 둠)

플러그인 (전체 단위, 수동):
  verify → 전체 스캔 → 5개 카테고리 리포트 (수정 없음)
  fix (dry_run) → 미리보기
  fix (적용) → Pass 1: 외부참조/URL 수정 → Pass 2: 고아 삭제
```

### 11.3 상호 보완 관계

내장 sync는 **편집 시점**에 실시간으로 동작하고,
플러그인은 **사후 점검 + 일괄 정리** 역할을 한다.

| 시나리오 | 내장 | 플러그인 |
|----------|------|----------|
| 다른 글에서 이미지 복사+붙여넣기 | 붙여넣기 시 즉시 해결 | — |
| 직접 타이핑한 외부참조 | 저장 시 해결 | fix로도 해결 가능 |
| 파일 이동 후 자기 링크 업데이트 | 이동 시 자동 | — |
| 파일 삭제 후 남은 고아 이미지 | 삭제 안함 | fix로 정리 |
| 대소문자 불일치 감지 | — | verify로 감지 |
| 깨진 링크 감지 | — | verify로 감지 |
| Non-Section 리소스 확인 | — | verify로 확인 |

### 11.4 고아 삭제 범위 차이

```
내장: 고아를 삭제하지 않음
  → 이유: 단일 파일 기준으로는 다른 글이 참조하는지 알 수 없음

플러그인 (fix): section 내 고아만 삭제
  → 전체 글 스캔 후 "어디서도 참조하지 않는" 이미지만 삭제
  → section 밖 파일(favicon, logo 등)은 삭제하지 않음

플러그인 (verify): 전체 image_path 스캔
  → section 내 고아: [!] Orphans로 표시 (fix 대상)
  → section 밖 파일: [i] Non-Section Files로 표시 (정보성)
```

---

## 구현 함수 요약

### Rust 내장 (file_service.rs)

| 함수 | 역할 |
|------|------|
| `image_abs(config, rel)` | 이미지 절대경로: `{base_path}/{image_path}/{rel}` |
| `strip_section_prefix(config, rel)` | 섹션 접두사 제거 (legacy 경로 변환) |
| `find_image_dir(sftp, config, rel)` | 새 경로 → legacy 경로 순서로 이미지 디렉토리 검색 |
| `parse_all_image_refs(content)` | md 내 이미지 참조 추출 (로컬 + 외부 URL, `![](...)` + `<img>`) |
| `copy_file_checked(sftp, src, dst)` | 해시 비교 복사 (Copied / Skipped / Renamed) |
| `download_url_to_sftp(sftp, url, dst)` | URL 다운로드 → SFTP 업로드 |
| `generate_url_filename(url)` | URL → sha256 해시 12자 + 확장자 파일명 |
| `replace_image_ref(content, old, new)` | md 내 특정 이미지 참조 치환 |
| `sync_images_on_save()` | 저장 시: 외부참조 복사 + 외부 URL 다운로드 |
| `sync_images_on_move()` | 이동 시: 이미지 참조 prefix 일괄 치환 |
| `update_image_refs_in_file()` | md 내 이미지 참조 경로 prefix 치환 (starts_with 기반) |
| `find_md_files_recursive()` | SFTP 폴더 내 모든 .md 파일 재귀 수집 |
| `read_file_bytes(sftp, path)` | SFTP 파일을 바이트로 읽기 |

### 플러그인 (Python, 서버 직접 실행)

| 플러그인 | 파일 | 역할 |
|----------|------|------|
| `fix-image-link` | `plugins/fix-image-link/main.py` | 전체 일괄: 외부참조 복사 + URL 다운로드 + 고아 삭제 (two-pass) |
| `verify-image-link` | `plugins/verify-image-link/main.py` | 전체 검증: 5개 카테고리 리포트, 수정 없음 |
