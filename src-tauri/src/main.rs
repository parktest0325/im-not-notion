// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod setting;
mod ssh;
mod utils;

use anyhow::Result;
use app::{
    get_file_content, get_file_list_, make_directory, move_file_or_folder, move_to_trashcan,
    new_content_for_hugo, remove_file, save_file_content, save_file_image, update_and_connect,
};
use setting::{load_config, save_config};

fn main() -> Result<()> {
    // SSH 연결 설정
    match load_config() {
        Ok(config) => {
            if let Err(e) = update_and_connect(config) {
                eprintln!("Failed to update and connect: {:?}", e);
            }
        }
        Err(e) => eprintln!("Failed to load config: {:?}", e),
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            update_and_connect,
            get_file_list_,
            get_file_content,
            save_file_content,
            save_file_image,
            new_content_for_hugo,
            move_file_or_folder,
            move_to_trashcan,
            make_directory,
            remove_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}