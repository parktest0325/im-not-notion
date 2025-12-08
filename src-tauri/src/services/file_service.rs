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

// children 을 IndexMap 으로 가진 구조체
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
