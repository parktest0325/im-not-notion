use tauri::ipc::{Channel, InvokeError};
use crate::services::pty_service;
use crate::services::config_service::get_app_config;
use crate::utils::IntoInvokeError;

#[tauri::command]
pub fn start_pty_cmd(cols: u32, rows: u32, on_event: Channel<String>) -> Result<(), InvokeError> {
    let config = get_app_config().into_invoke_err()?;
    let ssh = config.get_active_ssh_config()
        .ok_or_else(|| InvokeError::from("No active server SSH config".to_string()))?;

    // on_output 콜백: I/O 스레드 → Tauri Channel → 프론트엔드
    let output_fn = Box::new(move |text: String| -> bool {
        on_event.send(text).is_ok()
    });

    pty_service::start_pty(
        &ssh.host, &ssh.port, &ssh.username, &ssh.password,
        cols, rows, output_fn,
    ).into_invoke_err()?;

    Ok(())
}

#[tauri::command]
pub fn write_pty_cmd(data: String) -> Result<(), InvokeError> {
    pty_service::write_pty(data.as_bytes()).into_invoke_err()?;
    Ok(())
}

#[tauri::command]
pub fn resize_pty_cmd(cols: u32, rows: u32) -> Result<(), InvokeError> {
    pty_service::resize_pty(cols, rows).into_invoke_err()?;
    Ok(())
}

#[tauri::command]
pub fn stop_pty_cmd() -> Result<(), InvokeError> {
    pty_service::stop_pty().into_invoke_err()?;
    Ok(())
}
