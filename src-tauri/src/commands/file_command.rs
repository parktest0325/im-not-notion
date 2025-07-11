use crate::services::{
    config_service::get_hugo_config, execute_ssh_command, get_channel_session, get_file, get_file_list, get_sftp_session, move_file, rmrf_file, save_file, save_image, FileSystemNode, NodeType
};
use crate::services::file_service::mkdir_recursive;
use ssh2::Sftp;
use tauri::ipc::InvokeError;
use std::path::Path;

fn merge_hidden_files(main_list: &mut FileSystemNode, hidden_list: FileSystemNode, hidden_path: &str) {
    for hidden_child in hidden_list.children {
        merge_hidden_node(main_list, &hidden_child, &format!("/{}/", hidden_path));
    }
}

fn merge_hidden_node(main_list: &mut FileSystemNode, hidden_node: &FileSystemNode, prefix: &str) {
    // 디렉토리인 경우 재귀적으로 병합
    if matches!(hidden_node.type_, NodeType::Directory) {
        // 같은 이름의 디렉토리 찾기
        if let Some(existing_dir) = main_list.children.iter_mut().find(|child| 
            child.name == hidden_node.name && matches!(child.type_, NodeType::Directory)
        ) {
            // 기존 디렉토리에 Hidden 파일들 병합
            for child in &hidden_node.children {
                merge_hidden_node(existing_dir, child, &format!("{}{}/", prefix, hidden_node.name));
            }
        } else {
            // 새로운 디렉토리 생성 (Hidden 내용이 있을 때만)
            let mut new_dir = hidden_node.clone();
            update_hidden_paths(&mut new_dir, &format!("{}{}/", prefix, hidden_node.name));
            // 실제 내용이 있는 디렉토리만 추가
            if !new_dir.children.is_empty() {
                main_list.children.push(new_dir);
            }
        }
    } else {
        // 파일인 경우 원래 이름으로 표시하되 is_hidden을 true로 설정
        let mut hidden_file = hidden_node.clone();
        hidden_file.is_hidden = true;
        
        // 같은 이름의 파일이 이미 있는지 확인하고 중복 제거
        if !main_list.children.iter().any(|child| 
            child.name == hidden_file.name && matches!(child.type_, NodeType::File)
        ) {
            main_list.children.push(hidden_file);
        }
    }
}

fn update_hidden_paths(node: &mut FileSystemNode, prefix: &str) {
    for child in &mut node.children {
        if matches!(child.type_, NodeType::File) {
            child.is_hidden = true;
        } else {
            update_hidden_paths(child, &format!("{}{}/", prefix, child.name));
        }
    }
}

#[tauri::command]
pub fn get_file_list_() -> Result<FileSystemNode, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    
    // 일반 content 폴더 리스트 가져오기
    let mut file_list = get_file_list(
        &sftp,
        Path::new(&format!("{}/content/{}", &hugo_config.base_path, &hugo_config.content_path)),
        5,
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    
    
    // Hidden 폴더 리스트 가져오기 (있으면)
    let hidden_path_str = format!("{}/content/{}", &hugo_config.base_path, &hugo_config.hidden_path);
    let hidden_path = Path::new(&hidden_path_str);
    if sftp.stat(hidden_path).is_ok() {
        let hidden_list = get_file_list(&sftp, hidden_path, 5)
            .map_err(|e| InvokeError::from(e.to_string()))?;
        
        // Hidden 폴더의 파일들을 원래 위치에 병합
        merge_hidden_files(&mut file_list, hidden_list, &hugo_config.hidden_path);
    }
    
    Ok(file_list)
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    
    // Hidden 파일인지 확인하고 적절한 경로 생성
    let is_hidden_file = check_file_hidden(file_path).unwrap_or(false);
    let actual_path = if is_hidden_file {
        format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, file_path)
    } else {
        format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.content_path, file_path)
    };
    
    // 디버깅: 경로 출력
    println!("DEBUG - get_file_content:");
    println!("  file_path: {}", file_path);
    println!("  is_hidden_file: {}", is_hidden_file);
    println!("  base_path: {}", &hugo_config.base_path);
    println!("  actual_path: {}", &actual_path);
    
    let file_data = get_file(&sftp, Path::new(&actual_path))
        .map_err(|e| {
            println!("ERROR - Failed to read file: {}", e);
            InvokeError::from(e.to_string())
        })?;
    
    println!("SUCCESS - File read successfully, content length: {}", file_data.len());
    Ok(file_data)
}

#[tauri::command]
pub fn save_file_content(file_path: &str, file_data: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // Hidden 파일인지 확인하고 적절한 경로 생성
    let is_hidden_file = check_file_hidden(file_path).unwrap_or(false);
    let actual_path = if is_hidden_file {
        format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, file_path)
    } else {
        format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.content_path, file_path)
    };

    // 디버깅: 경로 출력
    println!("DEBUG - save_file_content:");
    println!("  file_path: {}", file_path);
    println!("  is_hidden_file: {}", is_hidden_file);
    println!("  base_path: {}", &hugo_config.base_path);
    println!("  actual_path: {}", &actual_path);
    println!("  data_length: {}", file_data.len());

    save_file(&sftp, Path::new(&actual_path), file_data.to_string())
        .map_err(|e| {
            println!("ERROR - Failed to save file: {}", e);
            InvokeError::from(e.to_string())
        })?;
    
    println!("SUCCESS - File saved successfully");
    Ok(())
}

#[tauri::command]
pub fn save_file_image(
    file_path: &str,
    file_name: &str,
    file_data: Vec<u8>,
) -> Result<String, InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;

    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    // TODO: extract image_ext from image raw data
    let image_ext = "";
    let ret_path = format!("{}/{}{}", file_path, file_name, image_ext);

    save_image(
        &sftp,
        Path::new(&format!(
            "{}/{}/{}",
            &hugo_config.base_path, &hugo_config.image_path, ret_path
        )),
        file_data,
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(ret_path)
}

#[tauri::command]
pub fn new_content_for_hugo(file_path: &str) -> Result<(), InvokeError> {
    let mut channel = get_channel_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    execute_ssh_command(
        &mut channel,
        &format!(
            "cd {} ; {} new {}/{}",
            &hugo_config.base_path,
            &hugo_config.hugo_cmd_path,
            &hugo_config.content_path,
            file_path,
        ),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn remove_file(path: &str) -> Result<(), InvokeError> {
    let mut channel = get_channel_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // 일반 파일/폴더 삭제
    let normal_path = format!(
        "{}/content/{}/{}",
        &hugo_config.base_path, &hugo_config.content_path, path
    );
    
    // Hidden 폴더에 있는 파일/폴더도 삭제
    let hidden_path = format!(
        "{}/content/{}/{}",
        &hugo_config.base_path, &hugo_config.hidden_path, path
    );
    
    println!("DEBUG - remove_file:");
    println!("  path: {}", path);
    println!("  normal_path: {}", normal_path);
    println!("  hidden_path: {}", hidden_path);
    
    // 일반 파일 삭제
    rmrf_file(&mut channel, &normal_path)
        .map_err(|e| {
            println!("ERROR - Failed to remove normal file: {}", e);
            InvokeError::from(e.to_string())
        })?;
    
    // Hidden 파일도 삭제 (존재하면)
    if let Err(e) = rmrf_file(&mut channel, &hidden_path) {
        println!("INFO - Hidden file not found or already removed: {}", e);
        // Hidden 파일이 없어도 오류로 처리하지 않음
    } else {
        println!("SUCCESS - Hidden file also removed");
    }
    
    println!("SUCCESS - File removal completed");
    Ok(())
}

#[tauri::command]
pub fn move_file_or_folder(src: &str, dst: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    move_file(
        &sftp,
        &Path::new(&format!(
            "{}/content/{}/{}",
            &hugo_config.base_path, &hugo_config.content_path, src
        )),
        &Path::new(&format!(
            "{}/content/{}/{}",
            &hugo_config.base_path, &hugo_config.content_path, dst
        )),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

// #[tauri::command]
// pub fn move_to_trashcan(path: &str) -> Result<(), InvokeError> {
//     let sftp: Sftp = get_sftp_session()?;
//     let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

//     move_file(
//         &sftp,
//         &Path::new(&format!(
//             "{}/{}/{}",
//             &hugo_config.base_path, &hugo_config.content_path, path
//         )),
//         &Path::new(&format!(
//             "{}/{}/{}",
//             &hugo_config.base_path, &hugo_config.trashcan_path, path
//         )),
//     )
//     .map_err(|e| InvokeError::from(e.to_string()))?;
//     Ok(())
// }

#[tauri::command]
pub fn hide_file(path: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // path는 /path/to/file.md 형태로 들어옴
    // content/posts/path/to/file.md -> content/hidden_path/path/to/file.md 로 이동
    let source_path = format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.content_path, path);
    let hidden_full_path_str = format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, path);
    let hidden_full_path = Path::new(&hidden_full_path_str);
    
    // 디버깅: 경로 출력
    println!("DEBUG - hide_file:");
    println!("  path: {}", path);
    println!("  content_path: {}", &hugo_config.content_path);
    println!("  source_path: {}", source_path);
    println!("  hidden_full_path: {}", hidden_full_path_str);
    
    // hidden 폴더 자동 생성
    if let Some(parent) = hidden_full_path.parent() {
        mkdir_recursive(&sftp, parent).map_err(|e| InvokeError::from(e.to_string()))?;
    }

    // content/posts/file.md -> content/Hidden/posts/file.md 로 이동
    move_file(
        &sftp,
        &Path::new(&source_path),
        hidden_full_path,
    )
    .map_err(|e| {
        println!("ERROR - Failed to hide file: {}", e);
        InvokeError::from(e.to_string())
    })?;
    
    println!("SUCCESS - File hidden successfully");
    Ok(())
}

#[tauri::command]
pub fn show_file(path: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // path는 /path/to/file.md 형태로 들어옴 (Hidden 파일)
    // content/hidden_path/path/to/file.md -> content/posts/path/to/file.md 로 이동
    let source_path = format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, path);
    let target_full_path = format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.content_path, path);

    // 디버깅: 경로 출력
    println!("DEBUG - show_file:");
    println!("  path: {}", path);
    println!("  content_path: {}", &hugo_config.content_path);
    println!("  source_path: {}", source_path);
    println!("  target_full_path: {}", target_full_path);

    // content/Hidden/posts/file.md -> content/posts/file.md 로 이동
    move_file(
        &sftp,
        &Path::new(&source_path),
        &Path::new(&target_full_path),
    )
    .map_err(|e| {
        println!("ERROR - Failed to show file: {}", e);
        InvokeError::from(e.to_string())
    })?;
    
    println!("SUCCESS - File shown successfully");
    Ok(())
}

#[tauri::command]
pub fn check_file_hidden(path: &str) -> Result<bool, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // Hidden 폴더에 파일이 있는지 확인
    let hidden_path = format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, path);
    match sftp.stat(Path::new(&hidden_path)) {
        Ok(_) => Ok(true),  // Hidden 폴더에 파일이 있음
        Err(_) => Ok(false), // Hidden 폴더에 파일이 없음
    }
}
