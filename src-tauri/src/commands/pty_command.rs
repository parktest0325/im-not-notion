use tauri::ipc::{Channel, InvokeError};
use crate::services::pty_service;
use crate::services::config_service::get_app_config;
use crate::utils::IntoInvokeError;
use std::thread;
use std::time::Duration;

#[tauri::command]
pub fn start_pty_cmd(cols: u32, rows: u32, on_event: Channel<String>) -> Result<(), InvokeError> {
    let config = get_app_config().into_invoke_err()?;
    let ssh = &config.ssh_config;
    pty_service::start_pty(&ssh.host, &ssh.port, &ssh.username, &ssh.password, cols, rows)
        .into_invoke_err()?;

    // 백그라운드 읽기 루프
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            if !pty_service::is_pty_running() {
                break;
            }

            match pty_service::read_pty(&mut buf) {
                Ok(n) if n > 0 => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    if on_event.send(text).is_err() {
                        break;
                    }
                }
                Ok(_) => {
                    // 데이터 없음 (WouldBlock) — 잠시 대기
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });

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
