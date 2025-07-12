use crate::services::{
    config_service::get_hugo_config, execute_ssh_command, get_channel_session, get_file, get_file_list, get_sftp_session, move_file, rmrf_file, save_file, save_image, FileSystemNode,
    merge_tree,
};
use ssh2::Sftp;
use tauri::ipc::InvokeError;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn get_file_list_() -> Result<FileSystemNode, InvokeError> {
    let sftp         = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config  = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // 1) 메인 트리
    let mut main_root = get_file_list(
        &sftp,
        Path::new(&format!("{}/content/{}", hugo_config.base_path, hugo_config.content_path)),
        5, false
    ).map_err(|e| InvokeError::from(e.to_string()))?;

    // 2) 숨김 트리 (있으면)
    let hidden_root_path = PathBuf::from(format!(
        "{}/content/{}/{}",
        hugo_config.base_path, hugo_config.hidden_path, hugo_config.content_path
    ));
    if sftp.stat(&hidden_root_path).is_ok() {
        let hidden_root = get_file_list(&sftp, &hidden_root_path, 5, true)
            .map_err(|e| InvokeError::from(e.to_string()))?;

        // 3) 병합
        merge_tree(&mut main_root, hidden_root);
    }

    Ok(main_root)
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    let sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    let file_data = get_file(
        &sftp,
        Path::new(&format!("{}/content/{}", &hugo_config.base_path, file_path)),
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
            "{}/content/{}",
            &hugo_config.base_path, file_path
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
    let mut sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // 이건 path를 전달해야함. 그런데 path가 Hidden 내부일수도 있음. 이런식으로 만들어야지 양쪽다 지우려고하면 안됨
    // 폴더인 경우는 어떻게하는데 폴더는 양쪽다 있을수밖에 없단말이야.. 
    // rmrf_file(
    //     &mut channel,
    //     &format!(
    //         "{}/content/{}/{}",
    //         &hugo_config.base_path, &hugo_config.content_path, path
    //     ),
    // )
    // .map_err(|e| InvokeError::from(e.to_string()))?;

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
        match rmrf_file(&mut sftp, std::path::Path::new(p)) {
            Ok(_) => {
                println!("deleted: {p}");
                removed = true;      // 적어도 하나 성공
            }
            Err(e) => {
                println!("failed:  {p} ({e})");
                last_err = Some(e);  // 마지막 실패 기록
            }
        }
    }

    if removed {
        Ok(())  // 둘 중 하나라도 지워졌으면 성공
    } else {
        Err(InvokeError::from(
            last_err.unwrap_or_else(|| anyhow::anyhow!("unknown delete error")).to_string(),
        ))
    }
}

#[tauri::command]
pub fn move_file_or_folder(src: &str, dst: &str) -> Result<(), InvokeError> {
    let sftp: Sftp = get_sftp_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;

    // 폴더는 무조건 hidden:false이기 때문에 공개된애들만 옮겨지고, hidden인 애들은 안옮겨진다.
    // 그래서 그냥 remove 로직처럼 둘다 옮기기로 함
    // src, dst를 받으면, 
    // content/contents/src -> content/contents/dst/ ,
    // content/hidden/contents/src -> content/hidden/contents/dst  로 옮겨야한다.
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
        match move_file(&sftp, &Path::new(&src_full), &Path::new(&dst_full)) {
            Ok(_) => {
                println!("moved: {src_full} -> {dst_full}");
                moved = true;      // 적어도 하나 성공
            }
            Err(e) => {
                println!("failed:  {src_full} ({e})");
                last_err = Some(e);  // 마지막 실패 기록
            }
        }
    }
    if moved {
        Ok(())  // 둘 중 하나라도 옮겼다면 성공
    } else {
        Err(InvokeError::from(
            last_err.unwrap_or_else(|| anyhow::anyhow!("unknown delete error")).to_string(),
        ))
    }
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
