use std::path::{Path, PathBuf};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use ssh2::Sftp;
use std::io::prelude::*;
use indexmap::IndexMap;

use typeshare::typeshare;

use crate::services::ssh_service::{get_sftp_session, get_channel_session, execute_ssh_command};
use crate::services::config_service::get_hugo_config;
use crate::types::config::cms_config::HugoConfig;

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
pub fn write_content(file_path: &str, data: &str) -> Result<()> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let content_path = hugo_config.content_abs(file_path);
    let hidden_path = hugo_config.hidden_abs(file_path);

    save_file(&sftp, Path::new(&content_path), data.to_string())
        .or_else(|_| save_file(&sftp, Path::new(&hidden_path), data.to_string()))
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
    Ok(unique_path)
}

/// 파일/폴더 삭제 (content + hidden 양쪽 시도)
pub fn remove_content(path: &str) -> Result<()> {
    let (mut sftp, hugo_config) = sftp_and_config()?;
    let targets = [hugo_config.content_abs(path), hugo_config.hidden_abs(path)];
    try_both(targets, |p| rmrf_file(&mut sftp, Path::new(&p)))
}

/// 파일/폴더 이동 (content + hidden 양쪽 시도)
pub fn move_content(src: &str, dst: &str) -> Result<()> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let combos = [
        (hugo_config.content_abs(src), hugo_config.content_abs(dst)),
        (hugo_config.hidden_abs(src), hugo_config.hidden_abs(dst)),
    ];
    try_both(combos, |(s, d)| move_file(&sftp, Path::new(&s), Path::new(&d)))
}

/// 숨김 상태 토글
pub fn toggle_hidden(path: &str, state: bool) -> Result<()> {
    let (sftp, hugo_config) = sftp_and_config()?;

    let (src, dst) = if state {
        (hugo_config.hidden_abs(path), hugo_config.content_abs(path))
    } else {
        (hugo_config.content_abs(path), hugo_config.hidden_abs(path))
    };

    move_file(&sftp, Path::new(&src), Path::new(&dst))
}

/// 숨김 상태 확인
pub fn check_hidden(path: &str) -> Result<bool> {
    let (sftp, hugo_config) = sftp_and_config()?;
    Ok(sftp.stat(Path::new(&hugo_config.hidden_abs(path))).is_ok())
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
