use crate::services::{
    config_service::get_hugo_config, execute_ssh_command, get_channel_session, get_file, get_file_list, get_sftp_session, move_file, rmrf_file, save_file, save_image, FileSystemNode
};
use ssh2::Sftp;
use tauri::InvokeError;
use std::path::Path;

#[tauri::command]
pub fn get_file_list_() -> Result<FileSystemNode, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    let file_list = get_file_list(
        &sftp,
        Path::new(&format!("{}/{}", &hugo_config.base_path, &hugo_config.content_path)),
        5,
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(file_list)
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    let file_data = get_file(
        &sftp,
        Path::new(&format!("{}/{}/{}", &hugo_config.base_path, &hugo_config.content_path, file_path)),
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(file_data)
}

#[tauri::command]
pub fn save_file_content(file_path: &str, file_data: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;

    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    save_file(
        &sftp,
        Path::new(&format!(
            "{}/{}/{}",
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
        &format!("cd {} ; {} new content {}",
            &hugo_config.base_path,
            &hugo_config.hugo_cmd_path,
            &format!(
                "{}/{}/{}",
                &hugo_config.base_path, &hugo_config.content_path, file_path
            )
        ),
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn remove_file(path: &str) -> Result<(), InvokeError> {
    let mut channel = get_channel_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    rmrf_file(
        &mut channel,
        &format!(
            "{}/{}/{}",
            &hugo_config.base_path, &hugo_config.content_path, path
        ),
    )
    .map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn move_file_or_folder(src: &str, dst: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    move_file(
        &sftp,
        &Path::new(&format!(
            "{}/{}/{}",
            &hugo_config.base_path, &hugo_config.content_path, src
        )),
        &Path::new(&format!(
            "{}/{}/{}",
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