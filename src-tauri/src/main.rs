// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod types;
mod utils;
mod services;

use anyhow::Result;
use tauri_plugin_shell::init as shell_init;
use commands::{
    file_command::{
        get_file_content, get_file_list_, move_file_or_folder,
        new_content_for_hugo, remove_file, save_file_content, save_file_image,
        toggle_hidden_file, check_file_hidden,
    },
    config_command::{load_config, save_config, get_config},
    ssh_command::{update_and_connect, kill_server, start_server, execute_ssh},
};

fn main() -> Result<()> {
    tauri::Builder::default()
        .plugin(shell_init())
        .setup(|_app| {
            // SSH 연결 설정
            match load_config() {
                Ok(config) => {
                    if let Err(e) = update_and_connect(config) {
                        eprintln!("Failed to update and connect: {:?}", e);
                    }
                }
                Err(e) => eprintln!("Failed to load config: {:?}", e),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            get_config,
            update_and_connect,
            get_file_list_,
            get_file_content,
            save_file_content,
            save_file_image,
            new_content_for_hugo,
            move_file_or_folder,
            remove_file,
            kill_server,
            start_server,
            execute_ssh,
            toggle_hidden_file,
            check_file_hidden,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
