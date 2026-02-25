// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod types;
mod utils;
mod services;

use std::sync::OnceLock;
use anyhow::Result;
use tauri::Emitter;
#[cfg(target_os = "macos")]
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri_plugin_shell::init as shell_init;
use tauri_plugin_dialog::init as dialog_init;

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn emit_hook_actions(results: Vec<types::plugin::PluginResult>) {
    let Some(handle) = APP_HANDLE.get() else { return };
    for result in results {
        for action in result.actions {
            let _ = handle.emit("plugin-hook-action", &action);
        }
    }
}
use commands::{
    file_command::{
        get_file_content, get_file_tree, move_file_or_folder,
        new_content_for_hugo, remove_file, save_file_content, save_file_image,
        toggle_hidden_file, check_file_hidden, download_remote_file, sync_pasted_refs,
    },
    config_command::{load_config, save_config, save_plugin_local_path, switch_server, check_connection},
    ssh_command::{kill_server, start_server, execute_ssh},
    setup_command::{
        check_prerequisites_cmd, check_hugo_installed_cmd,
        detect_server_platform_cmd, get_latest_hugo_version_cmd,
        install_hugo_cmd, generate_site_name_cmd,
        create_hugo_site_cmd, validate_hugo_project_cmd,
        git_init_site_cmd, install_theme_cmd,
    },
    pty_command::{start_pty_cmd, write_pty_cmd, resize_pty_cmd, stop_pty_cmd},
    plugin_command::{
        list_plugins, install_plugin, uninstall_plugin,
        enable_plugin, disable_plugin, run_plugin,
        register_plugin_cron, unregister_plugin_cron,
        list_registered_crons,
        pull_plugin, open_plugin_in_editor,
    },
};

fn main() -> Result<()> {
    tauri::Builder::default()
        .plugin(shell_init())
        .plugin(dialog_init())
        .setup(|app| {
            APP_HANDLE.set(app.handle().clone()).ok();

            // macOS: Edit 메뉴가 있어야 Cmd+C/V/X/A가 WebView에 전달됨
            #[cfg(target_os = "macos")]
            {
                let edit_menu = SubmenuBuilder::new(app, "Edit")
                    .undo()
                    .redo()
                    .separator()
                    .cut()
                    .copy()
                    .paste()
                    .select_all()
                    .build()?;
                let menu = MenuBuilder::new(app)
                    .item(&edit_menu)
                    .build()?;
                app.set_menu(menu)?;
            }

            // 앱 시작 시 설정 로드 (SSH 연결 포함)
            if let Err(e) = load_config() {
                eprintln!("Failed to load config: {:?}", e);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            save_plugin_local_path,
            switch_server,
            check_connection,
            get_file_tree,
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
            download_remote_file,
            sync_pasted_refs,
            check_prerequisites_cmd,
            check_hugo_installed_cmd,
            detect_server_platform_cmd,
            get_latest_hugo_version_cmd,
            install_hugo_cmd,
            generate_site_name_cmd,
            create_hugo_site_cmd,
            validate_hugo_project_cmd,
            git_init_site_cmd,
            install_theme_cmd,
            start_pty_cmd,
            write_pty_cmd,
            resize_pty_cmd,
            stop_pty_cmd,
            list_plugins,
            install_plugin,
            uninstall_plugin,
            enable_plugin,
            disable_plugin,
            run_plugin,
            register_plugin_cron,
            unregister_plugin_cron,
            list_registered_crons,
            pull_plugin,
            open_plugin_in_editor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
