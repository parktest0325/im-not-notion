use tauri::ipc::InvokeError;
use crate::services::config_service::get_hugo_config;
use crate::services::ssh_service::{get_channel_session, connect_ssh, execute_ssh_command};
use crate::types::config::AppConfig;

#[tauri::command]
pub fn update_and_connect(config: AppConfig) -> Result<(), InvokeError> {
    connect_ssh(&config).map_err(|e| InvokeError::from(e.to_string()))
}

// [Session(-39)] Channel can not be reused.. so need to reconnect ssh session.
#[tauri::command]
pub fn kill_server() -> Result<(), InvokeError> {
    let mut channel = get_channel_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    let pid = execute_ssh_command(
        &mut channel,
        &format!("ps -ef | grep '{} server' | grep -v grep | awk '{{print$2}}'", &hugo_config.hugo_cmd_path)
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    println!("pid: {}", pid);
    // let s = execute_ssh_command(&mut channel, "id").map_err(|e| InvokeError::from(e.to_string()))?;
    // println!("s: {}", s);
    channel = get_channel_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let s = execute_ssh_command(
        &mut channel,
        &format!("kill -9 {}", pid)
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    println!("s: {}", s);

    Ok(())
}

#[tauri::command]
pub fn start_server() -> Result<(), InvokeError> {
    let mut channel = get_channel_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let hugo_config = get_hugo_config().map_err(|e| InvokeError::from(e.to_string()))?;
    execute_ssh_command(
        &mut channel,
        // this command is waiting for more user input... so i added "2>&1 < /dev/null" for not hanging
        &format!("cd {} ; nohup {} server --liveReloadPort=443 --bind=0.0.0.0 > ./nohup.out 2>&1 < /dev/null &", hugo_config.base_path, hugo_config.hugo_cmd_path)
    ).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn execute_ssh(cmd: &str) -> Result<String, InvokeError> {
    let mut channel = get_channel_session().map_err(|e| InvokeError::from(e.to_string()))?;
    let res = execute_ssh_command(&mut channel, cmd).map_err(|e| InvokeError::from(e.to_string()))?;
    Ok(res)
}