// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ssh;

use anyhow::Result;
use ssh::Client;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    let output = match init(name) {
        Ok(client) => match client.execute("ls -la") {
            Ok(result) => result,
            Err(err) => return format!("execute error : {}", err),
        },
        Err(err) => return format!("init error : {}", err),
    };

    // 기능 구현 후 이걸 디버깅용 메시지로 사용
    format!("{}", output)
}

fn init(str: &str) -> Result<Client> {
    let mut s = str.split(":");
    let host = s.next().unwrap_or("");
    let port = s.next().unwrap_or("");
    let name = s.next().unwrap_or("");
    let pass = s.next().unwrap_or("");

    let mut client = Client::new(host, port, name, pass, "")?;
    client.connect()?;
    Ok(client)
}

fn main() -> Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
