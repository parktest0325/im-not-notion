use std::path::{Path, PathBuf};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use ssh2::Sftp;
use std::io::prelude::*;
use indexmap::IndexMap;
use rand::Rng;

use typeshare::typeshare;

use crate::services::ssh_service::{get_sftp_session, get_channel_session, execute_ssh_command};
use crate::services::config_service::get_hugo_config;
use crate::services::plugin_service;
use crate::types::config::cms_config::HugoConfig;
use crate::types::plugin::HookEvent;

// ============================================================
// 고수준 Hugo 파일 작업
// ============================================================

const FILE_TREE_MAX_DEPTH: usize = 5;

/// SFTP 세션 + Hugo 설정을 한 번에 가져옴
fn sftp_and_config() -> Result<(Sftp, HugoConfig)> {
    let sftp = get_sftp_session()?;
    let config = get_hugo_config()?;
    Ok((sftp, config))
}

/// content/hidden 양쪽 경로에서 작업 시도. 하나라도 성공하면 Ok, 모두 실패하면 마지막 에러 반환.
fn try_both<T, F>(items: impl IntoIterator<Item = T>, mut op: F) -> Result<()>
where
    F: FnMut(T) -> Result<()>,
{
    let mut last_err: Option<anyhow::Error> = None;
    let mut ok = false;
    for item in items {
        match op(item) {
            Ok(_) => { ok = true; }
            Err(e) => { last_err = Some(e); }
        }
    }
    if ok {
        Ok(())
    } else {
        bail!("{}", last_err.unwrap_or_else(|| anyhow::anyhow!("operation failed")))
    }
}

/// content/hidden 양쪽을 확인하여 중복되지 않는 경로를 반환.
/// 이미 존재하면 _1, _2, ... suffix를 붙인다.
fn find_unique_path(sftp: &Sftp, hugo_config: &HugoConfig, file_path: &str) -> String {
    let is_dir = file_path.ends_with("/_index.md");

    if is_dir {
        // e.g. "/new_folder/_index.md" → 디렉토리 "/new_folder" 중복 확인
        let dir_part = &file_path[..file_path.len() - "/_index.md".len()];
        let (parent, name) = match dir_part.rfind('/') {
            Some(pos) => (&dir_part[..=pos], &dir_part[pos + 1..]),
            None => ("", dir_part),
        };

        if !path_exists(sftp, hugo_config, dir_part) {
            return file_path.to_string();
        }

        for n in 1..1000 {
            let candidate = format!("{}{}_{}", parent, name, n);
            if !path_exists(sftp, hugo_config, &candidate) {
                return format!("{}/_index.md", candidate);
            }
        }
        // fallback (사실상 도달 불가)
        file_path.to_string()
    } else {
        // e.g. "/parent/new_file.md" → 파일 중복 확인
        let (parent, file_name) = match file_path.rfind('/') {
            Some(pos) => (&file_path[..=pos], &file_path[pos + 1..]),
            None => ("", file_path),
        };
        let (stem, ext) = match file_name.rfind('.') {
            Some(pos) => (&file_name[..pos], &file_name[pos..]),
            None => (file_name, ""),
        };

        if !path_exists(sftp, hugo_config, file_path) {
            return file_path.to_string();
        }

        for n in 1..1000 {
            let candidate = format!("{}{}_{}{}", parent, stem, n, ext);
            if !path_exists(sftp, hugo_config, &candidate) {
                return candidate;
            }
        }
        file_path.to_string()
    }
}

/// content 경로와 hidden 경로 양쪽 모두 존재 여부 확인
fn path_exists(sftp: &Sftp, hugo_config: &HugoConfig, rel_path: &str) -> bool {
    sftp.stat(Path::new(&hugo_config.content_abs(rel_path))).is_ok()
        || sftp.stat(Path::new(&hugo_config.hidden_abs(rel_path))).is_ok()
}

/// 파일 트리 구성: 섹션별 트리 반환 (content + hidden 병합)
pub fn build_file_tree() -> Result<Vec<FileSystemNode>> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let mut sections = Vec::new();

    for section in &hugo_config.content_paths {
        let section_path = format!("{}/content/{}", hugo_config.base_path, section);
        let mut tree = match get_file_list(&sftp, Path::new(&section_path), FILE_TREE_MAX_DEPTH, false) {
            Ok(t) => t,
            Err(_) => continue, // 섹션 디렉토리가 없으면 스킵
        };
        tree.name = section.clone(); // 루트 노드 이름 = 섹션명

        // hidden 병합
        let hidden_path = format!("{}/content/{}/{}", hugo_config.base_path, hugo_config.hidden_path, section);
        if sftp.stat(Path::new(&hidden_path)).is_ok() {
            if let Ok(hidden) = get_file_list(&sftp, Path::new(&hidden_path), FILE_TREE_MAX_DEPTH, true) {
                merge_tree(&mut tree, hidden);
            }
        }

        sections.push(tree);
    }

    Ok(sections)
}

/// 파일 내용 읽기 (relativeFilePath 기반)
pub fn read_content(file_path: &str) -> Result<String> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let content_path = hugo_config.content_abs(file_path);
    let hidden_path = hugo_config.hidden_abs(file_path);

    get_file(&sftp, Path::new(&content_path))
        .or_else(|_| get_file(&sftp, Path::new(&hidden_path)))
}

/// 파일 내용 저장 (relativeFilePath 기반)
/// manual=true: 수동 저장 → 이미지 sync + hooks 실행
/// manual=false: 자동 저장 → 순수 저장만
/// 반환값: sync 성공 여부 (true=전부 성공, false=저장은 됐지만 sync 실패)
pub fn write_content(file_path: &str, data: &str, manual: bool) -> Result<bool> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let content_path = hugo_config.content_abs(file_path);
    let hidden_path = hugo_config.hidden_abs(file_path);

    // hidden 파일이면 hidden 경로에 저장, 아니면 content 경로에 저장
    let save_path = if sftp.stat(Path::new(&hidden_path)).is_ok() {
        hidden_path
    } else {
        content_path
    };
    save_file(&sftp, Path::new(&save_path), data.to_string())?;

    if manual {
        // 이미지 정합성 동기화 (외부 참조 복사 + 고아 삭제)
        if let Err(e) = sync_images_on_save(&sftp, &hugo_config, file_path, data) {
            crate::emit_hook_actions(vec![crate::types::plugin::PluginResult {
                success: false,
                message: Some(format!("Image sync failed: {}", e)),
                error: None,
                actions: vec![crate::types::plugin::PluginAction::Toast {
                    message: format!("Image sync failed: {}", e),
                    toast_type: "warning".to_string(),
                }],
            }]);
            return Ok(false);
        }

        if let Ok(results) = plugin_service::run_hooks(
            HookEvent::AfterFileSave,
            serde_json::json!({ "path": file_path }),
        ) {
            crate::emit_hook_actions(results);
        }
    }

    Ok(true)
}

/// 이미지 저장
pub fn write_image(file_path: &str, file_name: &str, data: Vec<u8>) -> Result<String> {
    let (sftp, hugo_config) = sftp_and_config()?;

    // TODO: extract image_ext from image raw data
    let image_ext = "";
    let ret_path = format!("{}/{}{}", file_path, file_name, image_ext);

    save_image(
        &sftp,
        Path::new(&format!("{}/{}/{}", &hugo_config.base_path, &hugo_config.image_path, ret_path)),
        data,
    )?;
    Ok(ret_path)
}

/// Hugo 새 콘텐츠 생성 (중복 이름 자동 처리)
pub fn create_content(file_path: &str) -> Result<String> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let mut channel = get_channel_session()?;

    let unique_path = find_unique_path(&sftp, &hugo_config, file_path);

    // unique_path에 섹션이 포함됨: e.g. "/posts/my-post/_index.md"
    let clean_path = unique_path.trim_start_matches('/');
    execute_ssh_command(
        &mut channel,
        &format!(
            "cd {} ; {} new {}",
            &hugo_config.base_path,
            &hugo_config.hugo_cmd_path,
            clean_path,
        ),
    )?;

    if let Ok(results) = plugin_service::run_hooks(
        HookEvent::AfterFileCreate,
        serde_json::json!({ "path": &unique_path }),
    ) {
        crate::emit_hook_actions(results);
    }

    Ok(unique_path)
}

/// 파일/폴더 삭제 (content + hidden 양쪽 시도)
/// 이미지 디렉토리는 삭제하지 않음 (고아로 유지, 플러그인에서 정리)
pub fn remove_content(path: &str) -> Result<()> {
    let (mut sftp, hugo_config) = sftp_and_config()?;
    let targets = [hugo_config.content_abs(path), hugo_config.hidden_abs(path)];
    try_both(targets, |p| rmrf_file(&mut sftp, Path::new(&p)))?;

    if let Ok(results) = plugin_service::run_hooks(
        HookEvent::AfterFileDelete,
        serde_json::json!({ "path": path }),
    ) {
        crate::emit_hook_actions(results);
    }

    Ok(())
}

/// 파일/폴더 이동 (rename + 실패 시 rollback)
/// 1. content/hidden rename
/// 2. 이미지 디렉토리 rename → 실패 시 1 되돌리기
/// 3. 참조 업데이트 → 실패 시 2,1 되돌리기
pub fn move_content(src: &str, dst: &str) -> Result<()> {
    let (sftp, hugo_config) = sftp_and_config()?;

    if path_exists(&sftp, &hugo_config, dst) {
        bail!("Destination already exists: {}", dst);
    }

    // === Phase 1: Rename content/hidden ===
    let content_src = hugo_config.content_abs(src);
    let content_dst = hugo_config.content_abs(dst);
    let hidden_src = hugo_config.hidden_abs(src);
    let hidden_dst = hugo_config.hidden_abs(dst);

    let content_exists = sftp.stat(Path::new(&content_src)).is_ok();
    let hidden_exists = sftp.stat(Path::new(&hidden_src)).is_ok();

    if !content_exists && !hidden_exists {
        bail!("Source does not exist: {}", src);
    }

    if content_exists {
        move_file(&sftp, Path::new(&content_src), Path::new(&content_dst))?;
    }
    if hidden_exists {
        if let Err(e) = move_file(&sftp, Path::new(&hidden_src), Path::new(&hidden_dst)) {
            // 롤백: content rename 되돌리기
            if content_exists {
                move_file(&sftp, Path::new(&content_dst), Path::new(&content_src)).ok();
            }
            return Err(anyhow::anyhow!("Failed to move hidden: {}", e));
        }
    }

    // === Phase 2: Rename image directory ===
    if src != dst {
        let dst_img = image_abs(&hugo_config, dst);
        // 새 경로 → legacy 경로 순서로 이미지 디렉토리 검색
        let found_src_img = find_image_dir(&sftp, &hugo_config, src);

        if let Some((src_img, _is_legacy)) = &found_src_img {
            if let Err(e) = move_file(&sftp, Path::new(src_img), Path::new(&dst_img)) {
                // 롤백: content/hidden rename 되돌리기
                if hidden_exists {
                    move_file(&sftp, Path::new(&hidden_dst), Path::new(&hidden_src)).ok();
                }
                if content_exists {
                    move_file(&sftp, Path::new(&content_dst), Path::new(&content_src)).ok();
                }
                return Err(anyhow::anyhow!("Failed to move images: {}", e));
            }
        }

        // === Phase 3: Update image refs ===
        // legacy 이미지를 이동한 경우, md 참조도 legacy→new 로 업데이트 필요
        if let Err(e) = sync_images_on_move(&sftp, &hugo_config, src, dst) {
            // 롤백: 이미지 + content/hidden rename 되돌리기
            if let Some((src_img, _)) = &found_src_img {
                if sftp.stat(Path::new(&dst_img)).is_ok() {
                    move_file(&sftp, Path::new(&dst_img), Path::new(src_img)).ok();
                }
            }
            if hidden_exists {
                move_file(&sftp, Path::new(&hidden_dst), Path::new(&hidden_src)).ok();
            }
            if content_exists {
                move_file(&sftp, Path::new(&content_dst), Path::new(&content_src)).ok();
            }
            return Err(anyhow::anyhow!("Failed to update refs: {}", e));
        }

        // === Phase 3.5: 이동된 파일의 외부참조 sync ===
        // 이동 후 외부참조(다른 파일의 이미지)를 내 폴더로 복사 + 링크 수정
        // 실패해도 롤백 불필요 (복사된 이미지는 고아로 처리됨)
        if dst.ends_with(".md") {
            // 단일 파일 이동
            let file_content = read_content(dst).unwrap_or_default();
            if !file_content.is_empty() {
                let _ = sync_images_on_save(&sftp, &hugo_config, dst, &file_content);
            }
        } else {
            // 폴더 이동 → 하위 모든 md 파일에 sync
            let content_base = hugo_config.content_abs("");
            let hidden_base = hugo_config.hidden_abs("");
            let content_dir = hugo_config.content_abs(dst);
            let hidden_dir = hugo_config.hidden_abs(dst);
            let mut md_files = find_md_files_recursive(&sftp, Path::new(&content_dir)).unwrap_or_default();
            md_files.extend(find_md_files_recursive(&sftp, Path::new(&hidden_dir)).unwrap_or_default());

            for abs_md in md_files {
                let abs_str = abs_md.to_string_lossy();
                // hidden_base를 먼저 체크 (content_base보다 더 구체적인 prefix)
                let rel = if abs_str.starts_with(&hidden_base) {
                    abs_str.strip_prefix(&hidden_base).unwrap_or(&abs_str)
                } else if abs_str.starts_with(&content_base) {
                    abs_str.strip_prefix(&content_base).unwrap_or(&abs_str)
                } else {
                    continue;
                };
                let file_content = read_content(rel).unwrap_or_default();
                if !file_content.is_empty() {
                    let _ = sync_images_on_save(&sftp, &hugo_config, rel, &file_content);
                }
            }
        }
    }

    // === Hook ===
    if let Ok(results) = plugin_service::run_hooks(
        HookEvent::AfterFileMove,
        serde_json::json!({ "src": src, "dst": dst }),
    ) {
        crate::emit_hook_actions(results);
    }

    Ok(())
}

/// 숨김 상태 토글 (토글 전 대상 경로 존재 여부 체크)
pub fn toggle_hidden(path: &str, state: bool) -> Result<()> {
    let (sftp, hugo_config) = sftp_and_config()?;

    let (src, dst) = if state {
        (hugo_config.hidden_abs(path), hugo_config.content_abs(path))
    } else {
        (hugo_config.content_abs(path), hugo_config.hidden_abs(path))
    };

    if sftp.stat(Path::new(&dst)).is_ok() {
        bail!("Destination already exists: {}", dst);
    }

    move_file(&sftp, Path::new(&src), Path::new(&dst))
}

/// 숨김 상태 확인
pub fn check_hidden(path: &str) -> Result<bool> {
    let (sftp, hugo_config) = sftp_and_config()?;
    Ok(sftp.stat(Path::new(&hugo_config.hidden_abs(path))).is_ok())
}

// ============================================================
// 이미지 동기화
// ============================================================

/// 이미지 절대경로: {base_path}/{image_path}/{rel}
fn image_abs(config: &HugoConfig, rel: &str) -> String {
    format!("{}/{}/{}", config.base_path, config.image_path, rel.trim_start_matches('/'))
}

/// 경로에서 섹션 접두사를 제거하여 legacy 경로 반환
/// e.g. "/posts/my-post/_index.md" → Some("my-post/_index.md")
fn strip_section_prefix<'a>(config: &HugoConfig, rel: &'a str) -> Option<&'a str> {
    let clean = rel.trim_start_matches('/');
    for section in &config.content_paths {
        let prefix = format!("{}/", section);
        if clean.starts_with(&prefix) {
            return Some(&clean[prefix.len()..]);
        }
    }
    None
}

/// 이미지 디렉토리를 새 경로 → legacy 경로 순서로 검색
/// 반환: (found_abs_path, is_legacy)
fn find_image_dir(sftp: &Sftp, config: &HugoConfig, rel: &str) -> Option<(String, bool)> {
    // 새 경로 (섹션 포함): {image_path}/posts/my-post/_index.md
    let new_path = image_abs(config, rel);
    if sftp.stat(Path::new(&new_path)).is_ok() {
        return Some((new_path, false));
    }
    // Legacy 경로 (섹션 없음): {image_path}/my-post/_index.md
    if let Some(legacy_rel) = strip_section_prefix(config, rel) {
        let legacy_path = image_abs(config, legacy_rel);
        if sftp.stat(Path::new(&legacy_path)).is_ok() {
            return Some((legacy_path, true));
        }
    }
    None
}

/// SFTP로 폴더 내 모든 .md 파일의 절대경로를 재귀 수집
fn find_md_files_recursive(sftp: &Sftp, dir: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    let entries = match sftp.readdir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(result),
    };
    for (child_path, stat) in entries {
        if stat.is_dir() {
            result.extend(find_md_files_recursive(sftp, &child_path)?);
        } else if child_path.extension().and_then(|e| e.to_str()) == Some("md") {
            result.push(child_path);
        }
    }
    Ok(result)
}

/// md 파일 내 이미지 참조 경로만 치환 (old_prefix → new_prefix)
/// ![...](...) 패턴 안에서만 치환하여 본문 텍스트나 일반 링크는 건드리지 않음
/// 경로 시작 부분(prefix)만 매칭하여 부분 문자열 오매칭 방지
fn update_image_refs_in_file(sftp: &Sftp, config: &HugoConfig, rel_path: &str, old_prefix: &str, new_prefix: &str) -> Result<()> {
    let content_path = config.content_abs(rel_path);
    let hidden_path = config.hidden_abs(rel_path);

    let (abs_path, content) = get_file(sftp, Path::new(&content_path))
        .map(|c| (content_path, c))
        .or_else(|_| get_file(sftp, Path::new(&hidden_path)).map(|c| (hidden_path, c)))?;

    let old_with_slash = format!("{}/", old_prefix);

    // 경로의 시작 부분에서만 old_prefix를 new_prefix로 치환
    let replace_path_prefix = |path: &str| -> String {
        let stripped = path.trim_start_matches('/');
        if stripped.starts_with(&old_with_slash) || stripped == old_prefix {
            let rest = &stripped[old_prefix.len()..];
            format!("/{}{}", new_prefix, rest)
        } else {
            path.to_string()
        }
    };

    // ![alt](path) 패턴
    let md_re = regex::Regex::new(r"(!\[[^\]]*\]\()([^)]+)(\))").unwrap();
    let updated = md_re.replace_all(&content, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let path = replace_path_prefix(&caps[2]);
        let suffix = &caps[3];
        format!("{}{}{}", prefix, path, suffix)
    }).to_string();

    // <img src="path"> 패턴
    let img_re = regex::Regex::new(r#"(<img\s[^>]*src\s*=\s*["'])([^"']+)(["'])"#).unwrap();
    let updated = img_re.replace_all(&updated, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let path = replace_path_prefix(&caps[2]);
        let suffix = &caps[3];
        format!("{}{}{}", prefix, path, suffix)
    }).to_string();

    if content != updated {
        save_file(sftp, Path::new(&abs_path), updated)?;
    }
    Ok(())
}

/// 파일/폴더 이동 시 이미지 참조 업데이트 (이동된 파일/폴더 내부의 자기 참조만)
fn sync_images_on_move(sftp: &Sftp, config: &HugoConfig, src: &str, dst: &str) -> Result<()> {
    let old_prefix = src.trim_start_matches('/');
    let new_prefix = dst.trim_start_matches('/');
    // Legacy prefix (섹션 없는 old 참조): "posts/my-post" → "my-post"
    let legacy_old = strip_section_prefix(config, src).map(|s| s.to_string());

    let content_base = config.content_abs("");
    let hidden_base = config.hidden_abs("");

    let apply_refs = |rel_path: &str| {
        // 새 형식 prefix 치환: posts/old-name → posts/new-name
        let _ = update_image_refs_in_file(sftp, config, rel_path, old_prefix, new_prefix);
        // Legacy 형식 치환: old-name → new-name (섹션 없는 참조)
        if let Some(lo) = &legacy_old {
            if lo != old_prefix {
                let _ = update_image_refs_in_file(sftp, config, rel_path, lo, new_prefix);
            }
        }
    };

    let md_files = if dst.ends_with(".md") {
        apply_refs(dst);
        return Ok(());
    } else {
        let content_dir = config.content_abs(dst);
        let hidden_dir = config.hidden_abs(dst);
        let mut files = find_md_files_recursive(sftp, Path::new(&content_dir))?;
        files.extend(find_md_files_recursive(sftp, Path::new(&hidden_dir))?);
        files
    };

    for abs_md in md_files {
        let abs_str = abs_md.to_string_lossy();
        // hidden_base를 먼저 체크 (content_base보다 더 구체적인 prefix)
        let rel = if abs_str.starts_with(&hidden_base) {
            abs_str.strip_prefix(&hidden_base).unwrap_or(&abs_str)
        } else if abs_str.starts_with(&content_base) {
            abs_str.strip_prefix(&content_base).unwrap_or(&abs_str)
        } else {
            continue;
        };
        apply_refs(rel);
    }

    Ok(())
}

/// SFTP 파일 내용을 바이트로 읽기
fn read_file_bytes(sftp: &Sftp, path: &Path) -> Result<Vec<u8>> {
    let mut file = sftp.open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    Ok(data)
}

/// 해시 비교 복사: 대상에 이미 동일 파일이 있으면 스킵, 다르면 새 이름으로 복사
enum CopyResult {
    Copied,             // dst 없었음 → 복사 완료
    Skipped,            // dst 존재 + 내용 동일 → 복사 스킵
    Renamed(String),    // dst 존재 + 내용 다름 → 새 이름으로 복사
}

fn copy_file_checked(sftp: &Sftp, src: &Path, dst: &Path) -> Result<CopyResult> {
    let src_data = read_file_bytes(sftp, src)?;

    if sftp.stat(dst).is_ok() {
        // 대상 파일 존재 → 바이트 비교
        if let Ok(dst_data) = read_file_bytes(sftp, dst) {
            if src_data == dst_data {
                return Ok(CopyResult::Skipped);
            }
        }
        // 내용이 다름 → 새 UUID 이름 생성
        let mut rng = rand::thread_rng();
        let hex: String = (0..16).map(|_| format!("{:x}", rng.gen::<u8>())).collect();
        let new_name = hex;
        let new_dst = dst.parent().unwrap_or(Path::new("/")).join(&new_name);
        if let Some(parent) = new_dst.parent() {
            mkdir_recursive(sftp, parent)?;
        }
        let mut dst_file = sftp.create(&new_dst)?;
        dst_file.write_all(&src_data)?;
        return Ok(CopyResult::Renamed(new_name));
    }

    // 대상 없음 → 일반 복사
    if let Some(parent) = dst.parent() {
        mkdir_recursive(sftp, parent)?;
    }
    let mut dst_file = sftp.create(dst)?;
    dst_file.write_all(&src_data)?;
    Ok(CopyResult::Copied)
}

/// md 내 모든 이미지 참조 추출 (로컬 + 외부 URL)
fn parse_all_image_refs(content: &str) -> (Vec<String>, Vec<String>) {
    let mut local_refs = Vec::new();
    let mut external_urls = Vec::new();

    let md_re = regex::Regex::new(r"!\[[^\]]*\]\(([^)]+)\)").unwrap();
    for cap in md_re.captures_iter(content) {
        let path = cap[1].trim().to_string();
        if path.starts_with("http://") || path.starts_with("https://") {
            external_urls.push(path);
        } else {
            local_refs.push(path);
        }
    }

    let img_re = regex::Regex::new(r#"<img\s[^>]*src\s*=\s*["']([^"']+)["']"#).unwrap();
    for cap in img_re.captures_iter(content) {
        let path = cap[1].trim().to_string();
        if path.starts_with("http://") || path.starts_with("https://") {
            external_urls.push(path);
        } else {
            local_refs.push(path);
        }
    }

    (local_refs, external_urls)
}

/// URL에서 파일명 생성 (sha2 해시 12자 + 확장자)
fn generate_url_filename(url: &str) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(url.as_bytes());
    let hex: String = hash.iter().take(6).map(|b| format!("{:02x}", b)).collect();

    // URL에서 확장자 추출
    let path_part = url.split('?').next().unwrap_or(url).split('#').next().unwrap_or(url);
    let ext = Path::new(path_part)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !ext.is_empty() && ext.len() <= 5 {
        format!("{}.{}", hex, ext)
    } else {
        hex.to_string()
    }
}

/// URL에서 이미지를 다운로드하여 SFTP로 업로드
fn download_url_to_sftp(sftp: &Sftp, url: &str, dst: &Path) -> Result<()> {
    let resp = ureq::get(url).call()
        .map_err(|e| anyhow::anyhow!("Download failed: {}", e))?;

    let mut data = Vec::new();
    resp.into_reader().read_to_end(&mut data)?;

    if data.is_empty() {
        bail!("Downloaded empty content from {}", url);
    }

    if let Some(parent) = dst.parent() {
        mkdir_recursive(sftp, parent)?;
    }
    let mut dst_file = sftp.create(dst)?;
    dst_file.write_all(&data)?;
    Ok(())
}

/// md 내 이미지 참조를 치환하는 헬퍼
fn replace_image_ref(content: &str, old_ref: &str, new_ref: &str) -> String {
    let escaped = regex::escape(old_ref);
    // ![alt](path) 패턴
    let md_re = regex::Regex::new(&format!(r"(!\[[^\]]*\]\(){}(\))", escaped)).unwrap();
    let result = md_re.replace_all(content, format!("${{1}}{}${{2}}", new_ref)).to_string();
    // <img src="path"> 패턴
    let img_re = regex::Regex::new(&format!(r#"(<img\s[^>]*src\s*=\s*["']){}(["'])"#, escaped)).unwrap();
    img_re.replace_all(&result, format!("${{1}}{}${{2}}", new_ref)).to_string()
}

/// 저장 시 이미지 정합성 동기화:
/// 1. 외부 이미지 참조 → 내 디렉토리로 복사 + 참조 수정
/// 2. 외부 URL → 다운로드 + 내 디렉토리에 저장 + 참조 수정
/// 고아 이미지는 삭제하지 않음 (플러그인에서 별도 처리)
fn sync_images_on_save(sftp: &Sftp, config: &HugoConfig, file_path: &str, content: &str) -> Result<()> {
    if !file_path.ends_with(".md") {
        return Ok(());
    }

    let rel = file_path.trim_start_matches('/');
    let my_prefix = format!("{}/", rel);

    let (local_refs, external_urls) = parse_all_image_refs(content);
    let mut updated_content = content.to_string();
    let mut modified = false;

    // 1. 로컬 외부참조 → 내 디렉토리로 복사
    for img_ref in &local_refs {
        let ref_clean = img_ref.trim_start_matches('/');
        if ref_clean.starts_with(&my_prefix) {
            continue; // 이미 내 디렉토리
        }

        let filename = Path::new(ref_clean)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(ref_clean);
        let new_ref = format!("/{}{}", my_prefix, filename);
        let dst_abs = image_abs(config, &new_ref);
        let src_abs = image_abs(config, ref_clean);

        if sftp.stat(Path::new(&src_abs)).is_err() {
            // 원본 없으면 대상 확인: 대상도 없으면 스킵 (깨진 참조)
            if sftp.stat(Path::new(&dst_abs)).is_err() {
                continue;
            }
            // 대상에 이미 있으면 링크만 업데이트
        } else if src_abs != dst_abs {
            // 원본 존재 → 해시 비교 복사
            match copy_file_checked(sftp, Path::new(&src_abs), Path::new(&dst_abs)) {
                Ok(CopyResult::Renamed(new_name)) => {
                    // 이름 충돌 → 새 이름으로 링크 업데이트
                    let renamed_ref = format!("/{}{}", my_prefix, new_name);
                    updated_content = replace_image_ref(&updated_content, img_ref, &renamed_ref);
                    modified = true;
                    continue;
                }
                Ok(_) => {} // Copied 또는 Skipped → 아래에서 기본 링크 업데이트
                Err(_) => continue, // 복사 실패 → 스킵
            }
        }

        // 링크 업데이트 (항상 / prefix)
        updated_content = replace_image_ref(&updated_content, img_ref, &new_ref);
        modified = true;
    }

    // 2. 외부 URL → 다운로드 + 저장
    for url in &external_urls {
        let filename = generate_url_filename(url);
        let new_ref = format!("/{}{}", my_prefix, filename);
        let dst_abs = image_abs(config, &new_ref);

        // 이미 다운로드 되어있으면 스킵
        if sftp.stat(Path::new(&dst_abs)).is_ok() {
            updated_content = replace_image_ref(&updated_content, url, &new_ref);
            modified = true;
            continue;
        }

        if let Err(_) = download_url_to_sftp(sftp, url, Path::new(&dst_abs)) {
            continue; // 다운로드 실패 → 해당 URL 스킵
        }

        updated_content = replace_image_ref(&updated_content, url, &new_ref);
        modified = true;
    }

    // 3. 수정된 내용 저장 (hidden/content 판별)
    if modified {
        let content_path = config.content_abs(file_path);
        let hidden_path = config.hidden_abs(file_path);
        let save_path = if sftp.stat(Path::new(&hidden_path)).is_ok() {
            hidden_path
        } else {
            content_path
        };
        save_file(sftp, Path::new(&save_path), updated_content)?;
    }

    Ok(())
}

/// 붙여넣기 텍스트의 외부 이미지 참조를 처리하여 수정된 텍스트 반환
/// 이미지 파일 복사 + URL 다운로드만 수행, 파일 저장은 하지 않음
pub fn sync_pasted_refs(file_path: &str, pasted_text: &str) -> Result<String> {
    let (sftp, config) = sftp_and_config()?;

    if !file_path.ends_with(".md") {
        return Ok(pasted_text.to_string());
    }

    let rel = file_path.trim_start_matches('/');
    let my_prefix = format!("{}/", rel);

    let (local_refs, external_urls) = parse_all_image_refs(pasted_text);
    let mut updated = pasted_text.to_string();

    // 로컬 외부참조 → 내 디렉토리로 복사
    for img_ref in &local_refs {
        let ref_clean = img_ref.trim_start_matches('/');
        if ref_clean.starts_with(&my_prefix) {
            continue;
        }

        let filename = Path::new(ref_clean)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(ref_clean);
        let new_ref = format!("/{}{}", my_prefix, filename);
        let dst_abs = image_abs(&config, &new_ref);
        let src_abs = image_abs(&config, ref_clean);

        if sftp.stat(Path::new(&src_abs)).is_err() {
            if sftp.stat(Path::new(&dst_abs)).is_err() {
                continue;
            }
        } else if src_abs != dst_abs {
            match copy_file_checked(&sftp, Path::new(&src_abs), Path::new(&dst_abs)) {
                Ok(CopyResult::Renamed(new_name)) => {
                    let renamed_ref = format!("/{}{}", my_prefix, new_name);
                    updated = replace_image_ref(&updated, img_ref, &renamed_ref);
                    continue;
                }
                Ok(_) => {}
                Err(_) => continue,
            }
        }

        updated = replace_image_ref(&updated, img_ref, &new_ref);
    }

    // 외부 URL → 다운로드 + 저장
    for url in &external_urls {
        let filename = generate_url_filename(url);
        let new_ref = format!("/{}{}", my_prefix, filename);
        let dst_abs = image_abs(&config, &new_ref);

        if sftp.stat(Path::new(&dst_abs)).is_ok() {
            updated = replace_image_ref(&updated, url, &new_ref);
            continue;
        }

        if download_url_to_sftp(&sftp, url, Path::new(&dst_abs)).is_err() {
            continue;
        }

        updated = replace_image_ref(&updated, url, &new_ref);
    }

    Ok(updated)
}

// ============================================================
// 저수준 SFTP 작업
// ============================================================

fn serialize_values<S>(
    map: &IndexMap<String, FileSystemNode>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let vec: Vec<&FileSystemNode> = map.values().collect();
    vec.serialize(serializer)
}

// children 을 IndexMap 으로 가진 구조체
#[typeshare]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSystemNode {
    pub name:       String,
    pub type_:      NodeType,
    pub is_hidden:  bool,
    #[typeshare(serialized_as = "Vec<FileSystemNode>")]
    #[serde(serialize_with = "serialize_values")]
    pub children:   IndexMap<String, FileSystemNode>,
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeType {
    File,
    Directory,
}


// public_list(root) 에 hidden_list 를 병합한다.
// public 쪽의 FileSystemNode 에 추가하는 것
// hidden_list 는 이미 is_hidden=true 로 세팅되어 있다고 가정.
pub fn merge_tree(
    public_list: &mut FileSystemNode,
    hidden_list: FileSystemNode,
) {
    // ex) name: "test_outerdir", 
    //     hidden_child: ["test_innerdir", "test.md" ...]
    for (name, hidden_child) in hidden_list.children {
        match hidden_child.type_ {
            // hidden_child가 디렉터리인 경우
            NodeType::Directory => {
                // 만약 디렉터리가 없는 경우
                let entry = public_list
                    .children
                    .entry(name.clone())
                    // or_insert_with 를 사용하면 key가 없을때만 생성
                    .or_insert_with(||FileSystemNode {    
                        name,
                        type_: NodeType::Directory,
                        is_hidden: false,            // public에 없는 폴더라면 메인 우선이라 false 기본값
                        children: IndexMap::new(),
                    });
                // 하위 디렉터리 재귀 병합    
                merge_tree(entry, hidden_child);
            }
            // 파일인 경우
            NodeType::File => {
                // 파일 이름 중복 시 메인 우선, 없으면 숨김 파일 추가
                public_list.children.entry(name).or_insert(hidden_child);
            }
        }
    }
}


// 전달받은 경로에서 하위 파일(폴더) 리스트 depth만큼 검색해서 가져오기
pub fn get_file_list(sftp: &Sftp, path: &Path, depth: usize, hidden: bool) -> Result<FileSystemNode> {
    let name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if depth == 0 {
        return Ok(FileSystemNode {
            name,
            type_: NodeType::Directory,
            is_hidden: hidden,
            children: IndexMap::new(),
        });
    }

    let mut children = IndexMap::new();
    for (child_path, stat) in sftp.readdir(path)? {
        let node = if stat.is_dir() {
            get_file_list(sftp, &child_path, depth - 1, hidden)?
        } else {
            let child_name = child_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            FileSystemNode {
                name: child_name,
                type_: NodeType::File,
                is_hidden: hidden,
                children: IndexMap::new(),
            }
        };
        children.insert(node.name.clone(), node);
    }

    Ok(FileSystemNode {
        name,
        type_: NodeType::Directory,
        is_hidden: hidden,
        children,
    })
}


// 재귀적으로 폴더 생성
pub fn mkdir_recursive(sftp: &Sftp, path: &Path) -> Result<()> {
    let mut current_path = PathBuf::new();

    for component in path.components() {
        current_path.push(component);
        if sftp.stat(&current_path).is_err() {
            sftp.mkdir(&current_path, 0o755)?;
        }
    }
    Ok(())
}


// 폴더를 만든 후 그 위치로 파일(폴더) 이동
pub fn move_file(sftp: &Sftp, src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        match sftp.stat(parent) {
            Ok(_) => {},
            Err(_) => {
                mkdir_recursive(sftp, parent)?;
            }
        }
    }
    sftp.rename(src, dst, None)?;
    Ok(())
}


// 재귀적으로 모든 하위 파일(폴더) 삭제
pub fn rmrf_file(sftp: &mut ssh2::Sftp, remote: &Path) -> Result<()> {
    let meta = sftp.stat(remote)?;
    if meta.is_dir() {
        // 재귀
        for entry in sftp.readdir(remote)? {
            let (child, _) = entry;
            rmrf_file(sftp, &child)?;          // child 는 PathBuf
        }
        sftp.rmdir(remote)?;
    } else {
        sftp.unlink(remote)?;
    }
    Ok(())
}


pub fn get_file(sftp: &Sftp, path: &Path) -> Result<String> {
    let mut file = sftp.open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}


pub fn save_file(sftp: &Sftp, path: &Path, content: String) -> Result<()> {
    let mut file = sftp.create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}


/// 서버의 원격 파일을 로컬로 다운로드
pub fn download_remote(remote_path: &str, local_path: &str) -> Result<()> {
    let sftp = get_sftp_session()?;
    let mut remote_file = sftp.open(Path::new(remote_path))?;
    let mut data = Vec::new();
    remote_file.read_to_end(&mut data)?;
    std::fs::write(local_path, &data)?;
    Ok(())
}

pub fn save_image(sftp: &Sftp, path: &Path, image: Vec<u8>) -> Result<()> {
    if let Some(parent) = path.parent() {
        mkdir_recursive(sftp, parent)?;
    }
    let mut file: ssh2::File = sftp.create(path)?;
    file.write_all(&image)?;
    Ok(())
}
