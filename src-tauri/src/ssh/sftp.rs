use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use ssh2::{Channel, Sftp};
use std::io::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct FileSystemNode {
    name: String,
    type_: NodeType,
    children: Vec<FileSystemNode>,
}

#[derive(Serialize, Deserialize)]
enum NodeType {
    File,
    Directory,
}

pub fn list_directory(sftp: &Sftp, path: &Path, depth: usize) -> Result<FileSystemNode> {
    if depth == 0 {
        return Ok(FileSystemNode {
            name: path.to_string_lossy().into_owned(),
            type_: NodeType::Directory,
            children: vec![],
        });
    }

    let mut children = Vec::new();
    for entry in sftp.readdir(path)? {
        let (path, stat) = entry;
        let node = if stat.is_dir() {
            list_directory(sftp, &path, depth - 1)?
        } else {
            FileSystemNode {
                name: path.file_name().unwrap().to_str().unwrap().into(),
                type_: NodeType::File,
                children: vec![],
            }
        };
        children.push(node);
    }

    Ok(FileSystemNode {
        name: path.file_name().unwrap().to_str().unwrap().into(),
        type_: NodeType::Directory,
        children,
    })
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
    // 경로에서 디렉토리 부분만 추출
    if let Some(parent) = path.parent() {
        // 디렉토리가 존재하지 않는 경우 생성
        // ssh2에는 직접적인 디렉토리 존재 체크 함수가 없으므로, 무조건 시도
        // 실패시 에러 핸들링은 여러분의 요구사항에 맞게 조정
        sftp.mkdir(parent, 0o775).ok();
    }

    let mut file = sftp.create(path)?;
    file.write_all(&image)?;
    Ok(())
}

pub fn mkdir_recursive(sftp: &Sftp, path: &Path) -> Result<()> {
    let mut current_path = PathBuf::new();

    for component in path.components() {
        current_path.push(component);
        // 이미 존재하는지 확인
        if sftp.stat(&current_path).is_err() {
            sftp.mkdir(&current_path, 0o755)?;
        }
    }

    Ok(())
}

pub fn move_file(sftp: &Sftp, src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        // 디렉토리의 존재 여부를 확인
        match sftp.stat(parent) {
            Ok(_) => {} // 디렉토리가 존재하면 아무것도 하지 않음
            Err(_) => {
                // 디렉토리가 존재하지 않으면 생성
                mkdir_recursive(sftp, parent);
            }
        }
    }
    sftp.rename(src, dst, None)?;
    Ok(())
}

pub fn new_hugo_content(
    channel: &mut Channel,
    base: &str,
    hugo_cmd_path: &str,
    path: &str,
) -> Result<()> {
    channel.exec(&format!(
        "cd {} ; {} new content {}",
        base, hugo_cmd_path, path
    ))?;
    println!("cd {} ; hugo new content {}", base, path);
    let mut s = String::new();
    channel.stderr().read_to_string(&mut s)?;
    println!("Command stderr: {}", s);
    channel.read_to_string(&mut s)?;
    println!("Command stdout: {}", s);
    Ok(())
}

pub fn rmrf_file(channel: &mut Channel, path: &str) -> Result<()> {
    channel.exec(&format!("rm -rf {}", path))?;
    Ok(())
}
