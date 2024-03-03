// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod setting;
mod ssh;

use std::{hash::RandomState, io::ErrorKind, path::Path};

use anyhow::Result;
use setting::{save_config, SETTING_FILE_PATH};
use ssh::Client;

use crate::setting::{load_config, AppConfig};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    let parts: Vec<&str> = name.split("=").collect();
    let output = if parts[1] == "upload" {
        match init(parts[0]) {
            Ok(client) => match client.upload_file(parts[2], parts[3]) {
                Ok(_) => "upload success".to_string(),
                Err(err) => format!("upload error : {}", err),
            },
            Err(err) => format!("init error : {}", err),
        }
    } else {
        match init(parts[0]) {
            Ok(client) => match client.execute(parts[1]) {
                Ok(result) => result,
                Err(err) => format!("execute error : {}", err),
            },
            Err(err) => format!("init error : {}", err),
        }
    };
    // 기능 구현 후 디버깅용 메시지로 사용
    format!("{}", output)
}

fn init(input_str: &str) -> Result<Client> {
    let mut config = load_config(&Path::new(SETTING_FILE_PATH)).unwrap_or_default();

    let parts: Vec<&str> = input_str.splitn(5, ':').collect();

    if config.ssh_client.host.is_empty() && parts.len() > 0 {
        config.ssh_client.host = parts[0].to_string();
    }
    if config.ssh_client.port.is_empty() && parts.len() > 1 {
        config.ssh_client.port = parts[1].to_string();
    }
    if config.ssh_client.username.is_empty() && parts.len() > 2 {
        config.ssh_client.username = parts[2].to_string();
    }
    if config.ssh_client.password.is_empty() && parts.len() > 3 {
        config.ssh_client.password = parts[3].to_string();
    }
    if config.ssh_client.key_path.is_empty() && parts.len() > 4 {
        config.ssh_client.key_path = parts[4].to_string();
    }

    println!("{:#?}", &config);

    save_config(&config, &Path::new(SETTING_FILE_PATH))?;

    let mut client = Client::new(
        &config.ssh_client.host,
        &config.ssh_client.port,
        &config.ssh_client.username,
        &config.ssh_client.password,
        &config.ssh_client.key_path,
    )?;

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
