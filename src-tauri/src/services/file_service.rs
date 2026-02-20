use std::path::{Path, PathBuf};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use ssh2::Sftp;
use std::io::prelude::*;
use indexmap::IndexMap;

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

/// 파일 트리 구성: content + hidden 병합
pub fn build_file_tree() -> Result<FileSystemNode> {
    let (sftp, hugo_config) = sftp_and_config()?;

    let mut main_root = get_file_list(
        &sftp,
        Path::new(&hugo_config.content_abs("")),
        FILE_TREE_MAX_DEPTH, false
    )?;

    let hidden_root_path = PathBuf::from(hugo_config.hidden_abs(""));
    if sftp.stat(&hidden_root_path).is_ok() {
        let hidden_root = get_file_list(&sftp, &hidden_root_path, FILE_TREE_MAX_DEPTH, true)?;
        merge_tree(&mut main_root, hidden_root);
    }

    Ok(main_root)
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
pub fn write_content(file_path: &str, data: &str, manual: bool) -> Result<()> {
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
        sync_images_on_save(&sftp, &hugo_config, file_path, data).ok();

        if let Ok(results) = plugin_service::run_hooks(
            HookEvent::AfterFileSave,
            serde_json::json!({ "path": file_path }),
        ) {
            crate::emit_hook_actions(results);
        }
    }

    Ok(())
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

    execute_ssh_command(
        &mut channel,
        &format!(
            "cd {} ; {} new {}/{}",
            &hugo_config.base_path,
            &hugo_config.hugo_cmd_path,
            &hugo_config.content_path,
            unique_path,
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
pub fn remove_content(path: &str) -> Result<()> {
    let (mut sftp, hugo_config) = sftp_and_config()?;
    let targets = [hugo_config.content_abs(path), hugo_config.hidden_abs(path)];
    try_both(targets, |p| rmrf_file(&mut sftp, Path::new(&p)))?;

    // 이미지 디렉토리 정리
    cleanup_images_on_delete(&mut sftp, &hugo_config, path).ok();

    if let Ok(results) = plugin_service::run_hooks(
        HookEvent::AfterFileDelete,
        serde_json::json!({ "path": path }),
    ) {
        crate::emit_hook_actions(results);
    }

    Ok(())
}

/// 파일/폴더 이동 (copy-then-delete 트랜잭션 방식)
/// 1. dst에 복사 (content/hidden + 이미지)
/// 2. 참조 업데이트 (자기 참조 + 외부 참조)
/// 3. 전부 성공 시 src 삭제
/// 4. 실패 시 dst 복사본 정리, src 원본 유지
pub fn move_content(src: &str, dst: &str) -> Result<()> {
    let (mut sftp, hugo_config) = sftp_and_config()?;

    if path_exists(&sftp, &hugo_config, dst) {
        bail!("Destination already exists: {}", dst);
    }

    // === Phase 1: Copy content to dst ===
    let mut copied_paths: Vec<String> = Vec::new();

    let content_src = hugo_config.content_abs(src);
    let content_dst = hugo_config.content_abs(dst);
    let hidden_src = hugo_config.hidden_abs(src);
    let hidden_dst = hugo_config.hidden_abs(dst);

    let content_exists = sftp.stat(Path::new(&content_src)).is_ok();
    let hidden_exists = sftp.stat(Path::new(&hidden_src)).is_ok();

    if !content_exists && !hidden_exists {
        bail!("Source does not exist: {}", src);
    }

    // content 복사
    if content_exists {
        if let Err(e) = copy_file_or_dir(&sftp, Path::new(&content_src), Path::new(&content_dst)) {
            return Err(anyhow::anyhow!("Failed to copy content: {}", e));
        }
        copied_paths.push(content_dst.clone());
    }

    // hidden 복사
    if hidden_exists {
        if let Err(e) = copy_file_or_dir(&sftp, Path::new(&hidden_src), Path::new(&hidden_dst)) {
            // 롤백: content 복사본 정리
            for p in &copied_paths { rmrf_file(&mut sftp, Path::new(p)).ok(); }
            return Err(anyhow::anyhow!("Failed to copy hidden: {}", e));
        }
        copied_paths.push(hidden_dst.clone());
    }

    // === Phase 2: Copy image directory ===
    if src != dst {
        let src_img = image_abs(&hugo_config, src);
        let dst_img = image_abs(&hugo_config, dst);
        let img_exists = sftp.stat(Path::new(&src_img)).is_ok();

        if img_exists {
            if let Err(e) = copy_file_or_dir(&sftp, Path::new(&src_img), Path::new(&dst_img)) {
                // 롤백: content/hidden 복사본 정리
                for p in &copied_paths { rmrf_file(&mut sftp, Path::new(p)).ok(); }
                return Err(anyhow::anyhow!("Failed to copy images: {}", e));
            }
            copied_paths.push(dst_img);
        }

        // === Phase 3: Update image refs ===
        if let Err(e) = sync_images_on_move(&sftp, &hugo_config, src, dst) {
            // 롤백: 모든 복사본 정리
            for p in &copied_paths { rmrf_file(&mut sftp, Path::new(p)).ok(); }
            return Err(anyhow::anyhow!("Failed to update refs: {}", e));
        }
    }

    // === Phase 4: Delete originals (commit) ===
    if content_exists {
        rmrf_file(&mut sftp, Path::new(&content_src)).ok();
    }
    if hidden_exists {
        rmrf_file(&mut sftp, Path::new(&hidden_src)).ok();
    }
    if src != dst {
        let src_img = image_abs(&hugo_config, src);
        if sftp.stat(Path::new(&src_img)).is_ok() {
            rmrf_file(&mut sftp, Path::new(&src_img)).ok();
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

/// md 파일 내 이미지 참조 경로를 치환 (old_prefix → new_prefix)
/// content 또는 hidden 경로에서 파일을 찾아 읽기/쓰기
fn update_image_refs_in_file(sftp: &Sftp, config: &HugoConfig, rel_path: &str, old_prefix: &str, new_prefix: &str) -> Result<()> {
    let content_path = config.content_abs(rel_path);
    let hidden_path = config.hidden_abs(rel_path);

    let (abs_path, content) = get_file(sftp, Path::new(&content_path))
        .map(|c| (content_path, c))
        .or_else(|_| get_file(sftp, Path::new(&hidden_path)).map(|c| (hidden_path, c)))?;

    let updated = content.replace(old_prefix, new_prefix);
    if content != updated {
        save_file(sftp, Path::new(&abs_path), updated)?;
    }
    Ok(())
}

/// 파일/폴더 이동 시 모든 이미지 참조 업데이트
/// 1. 이동된 파일/폴더 내부의 자기 참조 업데이트
/// 2. 다른 모든 md 파일에서 이전 경로를 참조하는 외부 참조 업데이트
fn sync_images_on_move(sftp: &Sftp, config: &HugoConfig, src: &str, dst: &str) -> Result<()> {
    let old_prefix = src.trim_start_matches('/');
    let new_prefix = dst.trim_start_matches('/');

    // 전체 content + hidden 에서 모든 md 파일을 스캔하여 old_prefix → new_prefix 치환
    let content_root = config.content_abs("");
    let hidden_root = config.hidden_abs("");

    let mut all_md_files = find_md_files_recursive(sftp, Path::new(&content_root))?;
    all_md_files.extend(find_md_files_recursive(sftp, Path::new(&hidden_root))?);

    let content_base = config.content_abs("");
    let hidden_base = config.hidden_abs("");

    for abs_md in all_md_files {
        let abs_str = abs_md.to_string_lossy();
        let rel = if abs_str.starts_with(&content_base) {
            abs_str.strip_prefix(&content_base).unwrap_or(&abs_str)
        } else if abs_str.starts_with(&hidden_base) {
            abs_str.strip_prefix(&hidden_base).unwrap_or(&abs_str)
        } else {
            continue;
        };
        let _ = update_image_refs_in_file(sftp, config, rel, old_prefix, new_prefix);
    }

    Ok(())
}

/// md 내 로컬 이미지 참조 경로 추출 (외부 URL 제외)
fn parse_image_refs(content: &str) -> Vec<String> {
    let re = regex::Regex::new(r"!\[[^\]]*\]\(([^)]+)\)").unwrap();
    re.captures_iter(content)
        .filter_map(|cap| {
            let path = cap[1].trim();
            if path.starts_with("http://") || path.starts_with("https://") {
                None
            } else {
                Some(path.to_string())
            }
        })
        .collect()
}

/// SFTP로 파일 복사 (read → write)
fn copy_file(sftp: &Sftp, src: &Path, dst: &Path) -> Result<()> {
    let mut src_file = sftp.open(src)?;
    let mut data = Vec::new();
    src_file.read_to_end(&mut data)?;

    if let Some(parent) = dst.parent() {
        mkdir_recursive(sftp, parent)?;
    }
    let mut dst_file = sftp.create(dst)?;
    dst_file.write_all(&data)?;
    Ok(())
}

/// SFTP로 디렉토리 재귀 복사
fn copy_dir_recursive(sftp: &Sftp, src: &Path, dst: &Path) -> Result<()> {
    mkdir_recursive(sftp, dst)?;
    for (child_path, stat) in sftp.readdir(src)? {
        let child_name = child_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid child path"))?;
        let dst_child = dst.join(child_name);
        if stat.is_dir() {
            copy_dir_recursive(sftp, &child_path, &dst_child)?;
        } else {
            copy_file(sftp, &child_path, &dst_child)?;
        }
    }
    Ok(())
}

/// SFTP로 파일 또는 디렉토리를 복사 (존재하는 경우만)
fn copy_file_or_dir(sftp: &Sftp, src: &Path, dst: &Path) -> Result<()> {
    match sftp.stat(src) {
        Ok(stat) => {
            if stat.is_dir() {
                copy_dir_recursive(sftp, src, dst)
            } else {
                copy_file(sftp, src, dst)
            }
        }
        Err(_) => Ok(()), // 소스가 없으면 스킵
    }
}

/// 저장 시 이미지 정합성 동기화:
/// 1. 외부 이미지 참조 → 내 디렉토리로 복사 + 참조 수정
/// 2. 고아 이미지 삭제 (디렉토리에 있지만 참조 안 되는 파일)
fn sync_images_on_save(sftp: &Sftp, config: &HugoConfig, file_path: &str, content: &str) -> Result<()> {
    if !file_path.ends_with(".md") {
        return Ok(());
    }

    let rel = file_path.trim_start_matches('/');
    let my_prefix = format!("{}/", rel);
    let my_image_dir = image_abs(config, rel);

    let refs = parse_image_refs(content);
    let mut updated_content = content.to_string();
    let mut modified = false;

    // 1. 외부 이미지 참조 → 내 디렉토리로 복사
    for img_ref in &refs {
        let ref_clean = img_ref.trim_start_matches('/');
        if ref_clean.starts_with(&my_prefix) {
            continue; // 이미 내 디렉토리의 이미지
        }
        let src_abs = image_abs(config, ref_clean);
        if sftp.stat(Path::new(&src_abs)).is_err() {
            continue; // 원본 파일이 없으면 스킵
        }
        let filename = Path::new(ref_clean)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(ref_clean);
        // 원본 참조가 /로 시작하면 새 참조도 /로 시작
        let prefix = if img_ref.starts_with('/') { "/" } else { "" };
        let new_ref = format!("{}{}{}", prefix, my_prefix, filename);
        let dst_abs = image_abs(config, &new_ref);

        copy_file(sftp, Path::new(&src_abs), Path::new(&dst_abs))?;
        updated_content = updated_content.replace(img_ref, &new_ref);
        modified = true;
    }

    // 수정된 내용 저장 (hidden 파일이면 hidden 경로에 저장)
    if modified {
        let content_path = config.content_abs(file_path);
        let hidden_path = config.hidden_abs(file_path);
        let save_path = if sftp.stat(Path::new(&hidden_path)).is_ok() {
            hidden_path
        } else {
            content_path
        };
        save_file(sftp, Path::new(&save_path), updated_content.clone())?;
    }

    // 2. 고아 이미지 삭제
    let final_content = if modified { &updated_content } else { content };
    let final_refs: std::collections::HashSet<String> = parse_image_refs(final_content)
        .iter()
        .map(|r| {
            let clean = r.trim_start_matches('/');
            Path::new(clean)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(clean)
                .to_string()
        })
        .collect();

    if let Ok(entries) = sftp.readdir(Path::new(&my_image_dir)) {
        for (child_path, stat) in entries {
            if stat.is_dir() { continue; }
            let filename = child_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
                .to_string();
            if !final_refs.contains(&filename) {
                let _ = sftp.unlink(&child_path);
            }
        }
        // 디렉토리가 비었으면 삭제
        if let Ok(remaining) = sftp.readdir(Path::new(&my_image_dir)) {
            if remaining.is_empty() {
                let _ = sftp.rmdir(Path::new(&my_image_dir));
            }
        }
    }

    Ok(())
}

/// 파일/폴더 삭제 시 대응하는 이미지 디렉토리 삭제
fn cleanup_images_on_delete(sftp: &mut Sftp, config: &HugoConfig, path: &str) -> Result<()> {
    let img_dir = image_abs(config, path);

    if sftp.stat(Path::new(&img_dir)).is_ok() {
        rmrf_file(sftp, Path::new(&img_dir))?;
    }

    Ok(())
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


pub fn save_image(sftp: &Sftp, path: &Path, image: Vec<u8>) -> Result<()> {
    if let Some(parent) = path.parent() {
        mkdir_recursive(sftp, parent)?;
    }
    let mut file: ssh2::File = sftp.create(path)?;
    file.write_all(&image)?;
    Ok(())
}
