// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod setting;
mod ssh;

use anyhow::Result;
use app::{get_file_content, get_file_list, save_file_content, update_and_connect};
use setting::{load_config, save_config};

fn main() -> Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            update_and_connect,
            get_file_list,
            get_file_content,
            save_file_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
