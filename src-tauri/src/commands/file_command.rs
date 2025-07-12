use crate::services::{
    config_service::get_hugo_config, execute_ssh_command, get_channel_session, get_file, get_file_list, get_sftp_session, move_file, rmrf_file, save_file, save_image, FileSystemNode
};
use ssh2::Sftp;
use tauri::ipc::InvokeError;
use std::path::Path;

#[tauri::command]
pub fn get_file_list_() -> Result<FileSystemNode, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    let file_list = get_file_list(
        &sftp,
        Path::new(&format!("{}/content/{}", &hugo_config.base_path, &hugo_config.content_path)),
        5,
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(file_list)
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    let cleaned_file_path = file_path.trim_start_matches('/');

    let hidden_path = Path::new(&hugo_config.base_path)
        .join("content")
        .join(&hugo_config.hidden_path)
        .join(&hugo_config.content_path)
        .join(cleaned_file_path);

    let content_path = Path::new(&hugo_config.base_path)
        .join("content")
        .join(&hugo_config.content_path)
        .join(cleaned_file_path);

    // 먼저 hidden에서 시도
    match get_file(&sftp, &hidden_path) {
        Ok(data) => Ok(data),
        Err(_) => {
            // 실패하면 content에서 시도
            get_file(&sftp, &content_path)
                .map_err(|e| InvokeError::from(e.to_string()))
        }
    }
}

#[tauri::command]
pub fn save_file_content(file_path: &str, file_data: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;

    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    save_file(
        &sftp,
        Path::new(&format!(
            "{}/content/{}/{}",
            &hugo_config.base_path, &hugo_config.content_path, file_path
        )),
        file_data.to_string(),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
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

    // 이건 path를 전달해야함. 그런데 path가 Hidden 내부일수도 있음. 이런식으로 만들어야지 양쪽다 지우려고하면 안됨
    // 폴더인 경우는 어떻게하는데 폴더는 양쪽다 있을수밖에 없단말이야.. 
    rmrf_file(
        &mut channel,
        &format!(
            "{}/content/{}/{}",
            &hugo_config.base_path, &hugo_config.content_path, path
        ),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;

    // let normal_path = format!(
    //     "{}/content/{}/{}",
    //     &hugo_config.base_path, &hugo_config.content_path, path
    // );
    
    // // Hidden 폴더에 있는 파일/폴더도 삭제
    // let hidden_path = format!(
    //     "{}/content/{}/{}/{}",
    //     &hugo_config.base_path, &hugo_config.hidden_path, &hugo_config.content_path, path
    // );
    
    // println!("DEBUG - remove_file:");
    // println!("  path: {}", path);
    // println!("  normal_path: {}", normal_path);
    // println!("  hidden_path: {}", hidden_path);
    
    // // 일반 파일 삭제
    // rmrf_file(&mut channel, &normal_path)
    //     .map_err(|e| {
    //         println!("ERROR - Failed to remove normal file: {}", e);
    //         InvokeError::from(e.to_string())
    //     })?;
    
    // // Hidden 파일도 삭제 (존재하면)
    // if let Err(e) = rmrf_file(&mut channel, &hidden_path) {
    //     println!("INFO - Hidden file not found or already removed: {}", e);
    //     // Hidden 파일이 없어도 오류로 처리하지 않음
    // } else {
    //     println!("SUCCESS - Hidden file also removed");
    // }
    
    // println!("SUCCESS - File removal completed");
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

#[tauri::command]
pub fn toggle_hidden_file(path: &str, state: bool) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // path는 /path/to/file.md 형태로 들어옴
    // content/hidden_path/path/to/file.md <-> content/posts/path/to/file.md
    let hidden_file= format!("{}/content/{}/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, &hugo_config.content_path, path);
    let content_file= format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.content_path, path);

    let (src, dst) = if state {
        // 현재 숨겨짐 상태 → 표시 상태로 이동
        ( hidden_file, content_file)
    } else {
        // 현재 표시 상태 → 숨김 상태로 이동
        ( content_file, hidden_file)
    };

    // 디버깅: 경로 출력
    println!("DEBUG - show_file:");
    println!("  path: {}", path);
    println!("  content_path: {}", &hugo_config.content_path);
    println!("  src: {}", src);
    println!("  dst: {}", dst);

    move_file(
        &sftp,
        &Path::new(&src),
        &Path::new(&dst),
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
    let hidden_path = format!("{}/content/{}/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, &hugo_config.content_path, path);
    match sftp.stat(Path::new(&hidden_path)) {
        Ok(_) => Ok(true),  // Hidden 폴더에 파일이 있음
        Err(_) => Ok(false), // Hidden 폴더에 파일이 없음
    }
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

// #[tauri::command]
// pub fn make_directory(path: &str) -> Result<(), InvokeError> {
//     let sftp: Sftp = get_sftp_session()?;
//     let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
//     mkdir_recursive(
//         &sftp,
//         &Path::new(&format!(
//             "{}/{}/{}",
//             &hugo_config.base_path, &hugo_config.content_path, path
//         )),
//     )
//     .map_err(|e| InvokeError::from(e.to_string()))?;
//     Ok(())
// }
