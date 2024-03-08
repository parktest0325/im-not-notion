use std::path::Path;

use serde::{Deserialize, Serialize};
use ssh2::Sftp;

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

pub fn list_directory(
    sftp: &Sftp,
    path: &Path,
    depth: usize,
) -> Result<FileSystemNode, ssh2::Error> {
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
