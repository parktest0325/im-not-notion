use tauri::ipc::InvokeError;
use crate::services::{config_service::get_hugo_config, ssh_service::{self, get_channel_session, execute_ssh_command, SearchMatch}};
use crate::utils::IntoInvokeError;

#[tauri::command]
pub fn kill_server() -> Result<(), InvokeError> {
    let mut channel = get_channel_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;
    // pkill은 프로세스가 없어도 에러를 반환하지만, 무시해도 안전함
    let _ = execute_ssh_command(
        &mut channel,
        &format!("pkill -f '{} server'", &hugo_config.hugo_cmd_path)
    );
    Ok(())
}

#[tauri::command]
pub fn start_server() -> Result<(), InvokeError> {
    let mut channel = get_channel_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;
    execute_ssh_command(
        &mut channel,
        // Clean public/ before starting, then launch hugo in background
        &format!("cd {} && rm -rf public && nohup {} server --liveReloadPort=443 --bind=0.0.0.0 --baseURL {} --appendPort=false > ./nohup.out 2>&1 < /dev/null &", hugo_config.base_path, hugo_config.hugo_cmd_path, hugo_config.url)
    ).into_invoke_err()?;
    Ok(())
}

#[tauri::command]
pub fn check_server() -> Result<String, InvokeError> {
    let mut channel = get_channel_session().into_invoke_err()?;
    let hugo_config = get_hugo_config().into_invoke_err()?;
    // Check if hugo server process is running
    let result = execute_ssh_command(
        &mut channel,
        &format!("pgrep -f '{} server'", hugo_config.hugo_cmd_path)
    );
    match result {
        Ok(output) if !output.trim().is_empty() => Ok(output),
        _ => {
            // Process not running — read nohup.out for error details
            let mut channel2 = get_channel_session().into_invoke_err()?;
            let nohup = execute_ssh_command(
                &mut channel2,
                &format!("tail -100 {}/nohup.out 2>/dev/null || echo 'nohup.out not found'", hugo_config.base_path)
            ).unwrap_or_else(|_| "Failed to read nohup.out".to_string());
            Err(InvokeError::from(nohup))
        }
    }
}

#[tauri::command]
pub fn execute_ssh(cmd: &str) -> Result<String, InvokeError> {
    let mut channel = get_channel_session().into_invoke_err()?;
    let res = execute_ssh_command(&mut channel, cmd).into_invoke_err()?;
    Ok(res)
}

#[tauri::command]
pub fn search_content_cmd(query: String) -> Result<Vec<SearchMatch>, InvokeError> {
    ssh_service::search_content(&query).into_invoke_err()
}