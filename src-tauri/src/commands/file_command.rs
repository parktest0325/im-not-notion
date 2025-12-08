use crate::services::{
    config_service::get_hugo_config, execute_ssh_command, get_channel_session, get_file, get_file_list, get_sftp_session, move_file, rmrf_file, save_file, save_image, FileSystemNode,
    merge_tree,
};
use crate::utils::IntoInvokeError;
use tauri::ipc::InvokeError;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn get_file_list_() -> Result<FileSystemNode, InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    // 1) 메인 트리
    let mut main_root = get_file_list(
        &sftp,
        Path::new(&format!("{}/content/{}", hugo_config.base_path, hugo_config.content_path)),
        5, false
    ).into_invoke_err()?;

    // 2) 숨김 트리 (있으면)
    let hidden_root_path = PathBuf::from(format!(
        "{}/content/{}/{}",
        hugo_config.base_path, hugo_config.hidden_path, hugo_config.content_path
    ));
    if sftp.stat(&hidden_root_path).is_ok() {
        let hidden_root = get_file_list(&sftp, &hidden_root_path, 5, true)
            .into_invoke_err()?;

        // 3) 병합
        merge_tree(&mut main_root, hidden_root);
    }

    Ok(main_root)
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    let file_data = get_file(
        &sftp,
        Path::new(&format!("{}/content/{}", &hugo_config.base_path, file_path)),
    ).into_invoke_err()?;
    Ok(file_data)
}

#[tauri::command]
pub fn save_file_content(file_path: &str, file_data: &str) -> Result<(), InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    save_file(
        &sftp,
        Path::new(&format!(
            "{}/content/{}",
            &hugo_config.base_path, file_path
        )),
        file_data.to_string(),
    ).into_invoke_err()?;
    Ok(())
}

#[tauri::command]
pub fn save_file_image(
    file_path: &str,
    file_name: &str,
    file_data: Vec<u8>,
) -> Result<String, InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

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
    ).into_invoke_err()?;
    Ok(ret_path)
}

#[tauri::command]
pub fn new_content_for_hugo(file_path: &str) -> Result<(), InvokeError> {
    let mut channel = get_channel_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    execute_ssh_command(
        &mut channel,
        &format!(
            "cd {} ; {} new {}/{}",
            &hugo_config.base_path,
            &hugo_config.hugo_cmd_path,
            &hugo_config.content_path,
            file_path,
        ),
    ).into_invoke_err()?;
    Ok(())
}

#[tauri::command]
pub fn remove_file(path: &str) -> Result<(), InvokeError> {
    let mut sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    let normal_path = format!(
        "{}/content/{}{}",
        &hugo_config.base_path, &hugo_config.content_path, path
    );

    // Hidden 폴더에 있는 파일/폴더도 삭제
    let hidden_path = format!(
        "{}/content/{}/{}{}",
        &hugo_config.base_path, &hugo_config.hidden_path, &hugo_config.content_path, path
    );

    println!("DEBUG - remove_file:");
    println!("  path: {}", path);
    println!("  normal_path: {}", normal_path);
    println!("  hidden_path: {}", hidden_path);

    let mut last_err: Option<anyhow::Error> = None;
    let mut removed = false;

    for p in [&normal_path, &hidden_path] {
        match rmrf_file(&mut sftp, Path::new(p)) {
            Ok(_) => {
                println!("deleted: {p}");
                removed = true;
            }
            Err(e) => {
                println!("failed: {p} ({e})");
                last_err = Some(e);
            }
        }
    }

    if removed {
        Ok(())
    } else {
        Err(InvokeError::from(
            last_err.unwrap_or_else(|| anyhow::anyhow!("unknown delete error")).to_string(),
        ))
    }
}

#[tauri::command]
pub fn move_file_or_folder(src: &str, dst: &str) -> Result<(), InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    let combos = [
        ( // 일반 영역
          format!("{}/content/{}{}", hugo_config.base_path, hugo_config.content_path, src),
          format!("{}/content/{}{}", hugo_config.base_path, hugo_config.content_path, dst)
        ),
        ( // Hidden 영역
          format!("{}/content/{}/{}{}", hugo_config.base_path, hugo_config.hidden_path, hugo_config.content_path, src),
          format!("{}/content/{}/{}{}", hugo_config.base_path, hugo_config.hidden_path, hugo_config.content_path, dst)
        ),
    ];

    let mut moved = false;
    let mut last_err: Option<anyhow::Error> = None;

    for (src_full, dst_full) in combos {
        match move_file(&sftp, Path::new(&src_full), Path::new(&dst_full)) {
            Ok(_) => {
                println!("moved: {src_full} -> {dst_full}");
                moved = true;
            }
            Err(e) => {
                println!("failed: {src_full} ({e})");
                last_err = Some(e);
            }
        }
    }

    if moved {
        Ok(())
    } else {
        Err(InvokeError::from(
            last_err.unwrap_or_else(|| anyhow::anyhow!("unknown move error")).to_string(),
        ))
    }
}

#[tauri::command]
pub fn toggle_hidden_file(path: &str, state: bool) -> Result<(), InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    let hidden_file = format!("{}/content/{}/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, &hugo_config.content_path, path);
    let content_file = format!("{}/content/{}{}", &hugo_config.base_path, &hugo_config.content_path, path);

    let (src, dst) = if state {
        (hidden_file, content_file)
    } else {
        (content_file, hidden_file)
    };

    println!("DEBUG - toggle_hidden_file:");
    println!("  path: {}", path);
    println!("  src: {}", src);
    println!("  dst: {}", dst);

    move_file(&sftp, Path::new(&src), Path::new(&dst)).map_err(|e| {
        println!("ERROR - Failed to toggle file: {}", e);
        InvokeError::from(e.to_string())
    })?;

    println!("SUCCESS - File toggled successfully");
    Ok(())
}

#[tauri::command]
pub fn check_file_hidden(path: &str) -> Result<bool, InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;

    let hidden_path = format!("{}/content/{}/{}{}", &hugo_config.base_path, &hugo_config.hidden_path, &hugo_config.content_path, path);
    match sftp.stat(Path::new(&hidden_path)) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
