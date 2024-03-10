use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use ssh2::Sftp;
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
