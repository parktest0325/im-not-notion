# move_content SFTP 최적화 계획

> 대상 파일: `src-tauri/src/services/file_service.rs`
> 상태: 미구현 (개선안 문서화)

---

## 1. 현재 문제

단일 `.md` 파일 이동 시 **SFTP 왕복 약 15~18회** 발생.
네트워크 레이턴시 50ms 기준 750~900ms, 해외 서버면 더 느림.

---

## 2. 현재 SFTP 호출 상세 (단일 .md 파일 이동)

```
move_content("posts/a.md", "posts/b.md")

Phase 0: 사전 체크
  ① stat(content_dst)          — dst 존재 체크 (path_exists)
  ② stat(hidden_dst)           — dst 존재 체크 (path_exists)
  ③ stat(content_src)          — src 존재 체크
  ④ stat(hidden_src)           — src 존재 체크

Phase 1: content/hidden rename
  ⑤ stat(content_dst.parent)   — move_file 내부 parent 체크
  ⑥ rename(content_src → dst)  — 실제 이동

Phase 2: 이미지 디렉토리 rename
  ⑦ stat(new_image_path)       — find_image_dir (새 경로)
  ⑧ stat(legacy_image_path)    — find_image_dir (레거시)
  (이미지 디렉토리 있으면 +2: stat(parent) + rename)

Phase 3: 참조 업데이트 (sync_images_on_move → update_image_refs_in_file)
  ⑨ open(content_path)         — 파일 읽기 시도
  ⑩ read                       — 파일 내용 읽기
  (실패 시 ⑪ open(hidden_path) + ⑫ read 추가)
  ⑪ create + write             — prefix 치환 후 저장 (변경 있을 때)
  (legacy prefix도 있으면 한 사이클 더: open + read + create + write)

Phase 3.5: 외부참조 처리 (read_content → sync_images_on_save)
  ⑫ open + read                — ★ Phase 3에서 이미 write한 파일을 다시 read
  ⑬ 외부참조마다 stat × 2       — src_abs + dst_abs 존재 체크
  ⑭ stat(hidden_path)          — hidden 판별 (저장 경로 결정)
  ⑮ create + write             — 수정 내용 저장 (변경 있을 때)

합계: 15~18회 (외부참조 없는 기본 케이스)
```

---

## 3. 개선안: I/O와 로직 분리

### 3.1 핵심 아이디어

현재 Phase 3과 Phase 3.5는 같은 파일을 **각각 read → 변환 → write** 한다.
이 두 단계를 합쳐서 **1번 read → 2단계 변환 → 1번 write**로 줄인다.

```
현재:
  Phase 3:   read → prefix 치환 → write
  Phase 3.5: read → 외부참조 처리 → write    ← ★ 같은 파일을 또 read

개선:
  read (1번)
  → prefix 치환 (in-memory)
  → 외부참조 처리 (in-memory)
  → write (1번, 변경 있을 때만)
```

### 3.2 함수 리팩토링

`update_image_refs_in_file`은 `sync_images_on_move`에서만 호출됨 (확인 완료).
I/O를 하는 wrapper와 순수 변환 함수를 분리:

```rust
// ── 현재 구조 (I/O + 로직 혼합) ──

fn update_image_refs_in_file(sftp, config, path, old, new) -> Result<()>
//  내부: get_file() → regex 치환 → save_file()

fn sync_images_on_save(sftp, config, path, content) -> Result<()>
//  내부: 파싱 → 외부참조 복사 → stat(hidden) → save_file()


// ── 개선 구조 (I/O 분리) ──

// 순수 변환: String → String (SFTP 호출 없음)
fn update_image_refs(content: &str, old_prefix: &str, new_prefix: &str) -> String

// 외부참조 처리: content를 받아서 변환 후 반환 (SFTP 복사는 함)
fn resolve_external_refs(sftp, config, path, content: &str) -> Result<(String, bool)>
//  반환: (변환된 content, 변경 여부)

// move_content 내부에서 조합:
let content = read_file(path);                           // read 1번
let content = update_image_refs(&content, old, new);     // Phase 3 (I/O 없음)
let content = update_image_refs(&content, legacy, new);  // legacy (I/O 없음)
let (content, modified) = resolve_external_refs(...);    // Phase 3.5 (복사만)
if modified { save_file(path, content); }                // write 1번
```

### 3.3 `move_file` parent 체크 최적화

`move_file` 내부에서 매번 `stat(parent)` 후 `mkdir_recursive`를 호출하는데,
실제로 parent가 없는 경우는 **이미지 디렉토리 이동 시** 뿐이다.

```rust
// 현재: 모든 move_file 호출에서 stat(parent) (3회)
move_file(content)  → stat(parent) + rename  // parent는 항상 존재
move_file(hidden)   → stat(parent) + rename  // parent는 항상 존재
move_file(image)    → stat(parent) + rename  // parent가 없을 수 있음

// 개선 옵션 A: move_file에 ensure_parent 파라미터 추가
pub fn move_file(sftp, src, dst, ensure_parent: bool)

// 개선 옵션 B: sftp.rename 직접 호출 (content/hidden), move_file은 이미지만
sftp.rename(content_src, content_dst, None)?;  // parent 체크 불필요
move_file(image_src, image_dst)?;              // parent 체크 필요
```

---

## 4. 개선 후 SFTP 호출 예상

```
Phase 0: 사전 체크
  ① stat(content_dst)
  ② stat(hidden_dst)
  ③ stat(content_src)
  ④ stat(hidden_src)

Phase 1: content/hidden rename
  ⑤ rename(content)             — parent 체크 생략 (-1)

Phase 2: 이미지 디렉토리 rename
  ⑥ stat(new_image_path)
  ⑦ stat(legacy_image_path)
  (있으면 +1~2: stat(parent) + rename)

Phase 3+3.5 통합:
  ⑧ open + read                  — 파일 읽기 (1번)
  ⑨ prefix 치환 (in-memory)      — SFTP 호출 없음
  ⑩ 외부참조 처리 (복사 시 stat)  — 있을 때만
  ⑪ stat(hidden_path)            — 저장 경로 판별
  ⑫ create + write               — 저장 (1번, 변경 시만)

합계: 10~12회
```

| | 현재 | 개선 후 | 절감 |
|---|---|---|---|
| stat | ~10 | ~8 | -2 |
| open + read | 2~4 | 1 | -1~3 |
| create + write | 2 | 1 | -1 |
| rename | 1 | 1 | 0 |
| **합계** | **15~18** | **10~12** | **-5~6** |
| **예상 시간 (50ms)** | 750~900ms | 500~600ms | **-250~300ms** |

---

## 5. 확인 사항

### 5.1 `update_image_refs_in_file` 호출처 (확인 완료)

`sync_images_on_move` 내부에서만 호출됨 (line 509, 513).
다른 곳에서 사용하지 않으므로 안전하게 리팩토링 가능.

### 5.2 `sync_images_on_save` 호출처 (확인 완료)

| 호출처 | 위치 | 비고 |
|--------|------|------|
| `write_content(manual=true)` | line 162 | 수동 저장 시 — 변경 없음 |
| `move_content` (단일 파일) | line 329 | Phase 3.5 — 통합 대상 |
| `move_content` (폴더 내 파일) | line 352 | Phase 3.5 — 통합 대상 |

`write_content`에서의 호출은 그대로 유지.
`move_content`에서의 호출만 통합 로직으로 교체.

### 5.3 `move_file` parent 체크가 필요한 경우

- content/hidden 이동: **불필요** (같은 content/hidden 트리 안에서의 rename)
- 이미지 디렉토리 이동: **필요할 수 있음** (이미지 경로 구조가 다를 수 있음)
- `toggle_hidden` (line 383): content↔hidden 간 이동이므로 **필요할 수 있음**

→ `move_file`의 parent 체크를 무조건 제거하면 안 됨.
→ 옵션 A (파라미터 추가)가 안전.

### 5.4 폴더 이동 시 Phase 3 + 3.5 통합

폴더 이동 시 `sync_images_on_move`는 하위 모든 .md를 `find_md_files_recursive`로 수집하고
각 파일마다 `update_image_refs_in_file`을 호출한 뒤,
`move_content`에서 다시 `find_md_files_recursive` + `read_content` + `sync_images_on_save`를 호출.

→ `find_md_files_recursive`도 2번 호출됨 (각각 readdir 재귀).
→ 폴더 이동의 경우 절감 효과가 파일 수에 비례하여 더 큼.

### 5.5 롤백 로직 영향

현재 Phase 3 실패 시 Phase 1, 2를 롤백한다.
Phase 3.5 실패 시에는 롤백하지 않는다 (고아만 남김).

통합 시에도 이 정책을 유지해야 함:
- prefix 치환 실패 → 롤백
- 외부참조 처리 실패 → 롤백 안함 (기존 정책)

통합 함수 내에서 prefix 치환은 순수 문자열 연산이므로 실패할 수 없음.
따라서 실질적으로 롤백 로직에 영향 없음.

---

## 6. 구현 순서 (제안)

1. `update_image_refs` 순수 함수 추출 (I/O 제거)
2. `resolve_external_refs` 함수 추출 (`sync_images_on_save`에서 I/O 분리)
3. `move_content` 내부에서 Phase 3 + 3.5 통합
4. `move_file`에 `ensure_parent` 파라미터 추가
5. 폴더 이동 시 `find_md_files_recursive` 1회로 통합
6. 빌드 검증 + 테스트
