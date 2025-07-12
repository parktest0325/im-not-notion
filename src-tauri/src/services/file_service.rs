use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use ssh2::{Channel, Sftp};
use std::io::prelude::*;
use indexmap::IndexMap;

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

/// children 을 IndexMap 으로 가진 구조체 예시
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSystemNode {
    pub name:       String,
    pub type_:      NodeType,
    pub is_hidden:  bool,
    #[serde(serialize_with = "serialize_values")]
    pub children:   IndexMap<String, FileSystemNode>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeType {
    File,
    Directory,
}

/// main_list(root) 에 hidden_list 를 병합한다.
/// hidden_list 는 이미 is_hidden=true 로 세팅되어 있다고 가정.
pub fn merge_tree(
    main_list: &mut FileSystemNode,
    hidden_list: FileSystemNode,
) {
    for (name, hidden_child) in hidden_list.children {
        match hidden_child.type_ {
            NodeType::Directory => {
                // 하위 디렉터리 재귀 병합
                let entry = main_list
                    .children
                    .entry(name.clone())
                    .or_insert_with(|| FileSystemNode {
                        name,
                        type_: NodeType::Directory,
                        is_hidden: false,            // 메인에 없던 폴더면 기본값
                        children: IndexMap::new(),
                    });
                merge_tree(entry, hidden_child);
            }
            NodeType::File => {
                // 파일 이름 중복 시 메인 우선, 없으면 숨김 파일 추가
                main_list.children.entry(name).or_insert(hidden_child);
            }
        }
    }
}

pub fn get_file_list(sftp: &Sftp, path: &Path, depth: usize, hidden: bool) -> Result<FileSystemNode> {
    if depth == 0 {
        return Ok(FileSystemNode {
            name: path.to_string_lossy().into_owned(),
            type_: NodeType::Directory,
            is_hidden: hidden,
            children: IndexMap::new(),
        });
    }

    let mut children = IndexMap::new();
    for entry in sftp.readdir(path)? {
        let (path, stat) = entry;
        let node: FileSystemNode = if stat.is_dir() {
            get_file_list(sftp, &path, depth - 1, hidden)?
        } else {
            FileSystemNode {
                name: path.file_name().unwrap().to_str().unwrap().into(),
                type_: NodeType::File,
                is_hidden: hidden,
                children: IndexMap::new(),
            }
        };
        children.insert(node.name.clone(), node);
    }

    Ok(FileSystemNode {
        name: path.file_name().unwrap().to_str().unwrap().into(),
        type_: NodeType::Directory,
        is_hidden: hidden,
        children,
    })
}

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

pub fn get_file(sftp: &Sftp, path: &Path) -> Result<String> {
    let mut file = sftp.open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    println!("content: {:#?}", content);
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
