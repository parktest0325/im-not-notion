use crate::services::{
    config_service::get_hugo_config, execute_ssh_command, get_channel_session, get_file, get_file_list, get_sftp_session, move_file, rmrf_file, save_file, save_image, FileSystemNode,
    merge_tree,
};
use crate::types::config::cms_config::HugoConfig;
use crate::utils::IntoInvokeError;
use tauri::ipc::InvokeError;
use ssh2::Sftp;
use std::path::{Path, PathBuf};

const FILE_TREE_MAX_DEPTH: usize = 5;

/// content/hidden 양쪽 경로에서 작업 시도. 하나라도 성공하면 Ok, 모두 실패하면 마지막 에러 반환.
fn try_both<T, F>(items: impl IntoIterator<Item = T>, mut op: F) -> Result<(), InvokeError>
where
    F: FnMut(T) -> anyhow::Result<()>,
{
    let mut last_err: Option<anyhow::Error> = None;
    let mut ok = false;
    for item in items {
        match op(item) {
            Ok(_) => { ok = true; }
            Err(e) => { last_err = Some(e); }
        }
    }
    if ok {
        Ok(())
    } else {
        Err(InvokeError::from(
            last_err.unwrap_or_else(|| anyhow::anyhow!("operation failed")).to_string(),
        ))
    }
}

/// SFTP 세션 + Hugo 설정을 한 번에 가져옴
fn sftp_and_config() -> Result<(Sftp, HugoConfig), InvokeError> {
    let sftp = get_sftp_session().into_invoke_err()?;
    let config = get_hugo_config().into_invoke_err()?;
    Ok((sftp, config))
}

#[tauri::command]
pub fn get_file_tree() -> Result<FileSystemNode, InvokeError> {
    let (sftp, hugo_config) = sftp_and_config()?;

    // 1) 메인 트리
    let mut main_root = get_file_list(
        &sftp,
        Path::new(&hugo_config.content_abs("")),
        FILE_TREE_MAX_DEPTH, false
    ).into_invoke_err()?;

    // 2) 숨김 트리 (있으면)
    let hidden_root_path = PathBuf::from(hugo_config.hidden_abs(""));
    if sftp.stat(&hidden_root_path).is_ok() {
        let hidden_root = get_file_list(&sftp, &hidden_root_path, FILE_TREE_MAX_DEPTH, true)
            .into_invoke_err()?;

        // 3) 병합
        merge_tree(&mut main_root, hidden_root);
    }

    Ok(main_root)
}

#[tauri::command]
pub fn get_file_content(file_path: &str) -> Result<String, InvokeError> {
    let (sftp, hugo_config) = sftp_and_config()?;

    // file_path는 프론트엔드에서 content_path를 이미 포함한 상태로 전달됨
    let file_data = get_file(
        &sftp,
        Path::new(&format!("{}/content/{}", &hugo_config.base_path, file_path)),
    ).into_invoke_err()?;
    Ok(file_data)
}

#[tauri::command]
pub fn save_file_content(file_path: &str, file_data: &str) -> Result<(), InvokeError> {
    let (sftp, hugo_config) = sftp_and_config()?;

    // file_path는 프론트엔드에서 content_path를 이미 포함한 상태로 전달됨
    save_file(
        &sftp,
        Path::new(&format!("{}/content/{}", &hugo_config.base_path, file_path)),
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
    let (sftp, hugo_config) = sftp_and_config()?;

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

/// content/hidden 양쪽을 확인하여 중복되지 않는 경로를 반환.
/// 이미 존재하면 _1, _2, ... suffix를 붙인다.
fn find_unique_path(sftp: &Sftp, hugo_config: &HugoConfig, file_path: &str) -> String {
    let is_dir = file_path.ends_with("/_index.md");

    if is_dir {
        // e.g. "/new_folder/_index.md" → 디렉토리 "/new_folder" 중복 확인
        let dir_part = &file_path[..file_path.len() - "/_index.md".len()];
        let (parent, name) = match dir_part.rfind('/') {
            Some(pos) => (&dir_part[..=pos], &dir_part[pos + 1..]),
            None => ("", dir_part),
        };

        if !path_exists(sftp, hugo_config, dir_part) {
            return file_path.to_string();
        }

        for n in 1..1000 {
            let candidate = format!("{}{}_{}", parent, name, n);
            if !path_exists(sftp, hugo_config, &candidate) {
                return format!("{}/_index.md", candidate);
            }
        }
        // fallback (사실상 도달 불가)
        file_path.to_string()
    } else {
        // e.g. "/parent/new_file.md" → 파일 중복 확인
        let (parent, file_name) = match file_path.rfind('/') {
            Some(pos) => (&file_path[..=pos], &file_path[pos + 1..]),
            None => ("", file_path),
        };
        let (stem, ext) = match file_name.rfind('.') {
            Some(pos) => (&file_name[..pos], &file_name[pos..]),
            None => (file_name, ""),
        };

        if !path_exists(sftp, hugo_config, file_path) {
            return file_path.to_string();
        }

        for n in 1..1000 {
            let candidate = format!("{}{}_{}{}", parent, stem, n, ext);
            if !path_exists(sftp, hugo_config, &candidate) {
                return candidate;
            }
        }
        file_path.to_string()
    }
}

/// content 경로와 hidden 경로 양쪽 모두 존재 여부 확인
fn path_exists(sftp: &Sftp, hugo_config: &HugoConfig, rel_path: &str) -> bool {
    sftp.stat(Path::new(&hugo_config.content_abs(rel_path))).is_ok()
        || sftp.stat(Path::new(&hugo_config.hidden_abs(rel_path))).is_ok()
}

#[tauri::command]
pub fn new_content_for_hugo(file_path: &str) -> Result<String, InvokeError> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let mut channel = get_channel_session().into_invoke_err()?;

    let unique_path = find_unique_path(&sftp, &hugo_config, file_path);

    execute_ssh_command(
        &mut channel,
        &format!(
            "cd {} ; {} new {}/{}",
            &hugo_config.base_path,
            &hugo_config.hugo_cmd_path,
            &hugo_config.content_path,
            unique_path,
        ),
    ).into_invoke_err()?;
    Ok(unique_path)
}

#[tauri::command]
pub fn remove_file(path: &str) -> Result<(), InvokeError> {
    let (mut sftp, hugo_config) = sftp_and_config()?;
    let targets = [hugo_config.content_abs(path), hugo_config.hidden_abs(path)];
    try_both(targets, |p| rmrf_file(&mut sftp, Path::new(&p)))
}

#[tauri::command]
pub fn move_file_or_folder(src: &str, dst: &str) -> Result<(), InvokeError> {
    let (sftp, hugo_config) = sftp_and_config()?;
    let combos = [
        (hugo_config.content_abs(src), hugo_config.content_abs(dst)),
        (hugo_config.hidden_abs(src), hugo_config.hidden_abs(dst)),
    ];
    try_both(combos, |(s, d)| move_file(&sftp, Path::new(&s), Path::new(&d)))
}

#[tauri::command]
pub fn toggle_hidden_file(path: &str, state: bool) -> Result<(), InvokeError> {
    let (sftp, hugo_config) = sftp_and_config()?;

    let (src, dst) = if state {
        (hugo_config.hidden_abs(path), hugo_config.content_abs(path))
    } else {
        (hugo_config.content_abs(path), hugo_config.hidden_abs(path))
    };

    move_file(&sftp, Path::new(&src), Path::new(&dst)).into_invoke_err()?;
    Ok(())
}

#[tauri::command]
pub fn check_file_hidden(path: &str) -> Result<bool, InvokeError> {
    let (sftp, hugo_config) = sftp_and_config()?;

    match sftp.stat(Path::new(&hugo_config.hidden_abs(path))) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
