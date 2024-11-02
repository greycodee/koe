mod ffmpeg;
mod voice_decode;

use std::fs;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn read_file(
    app_handle: AppHandle,
    path: &str,
    on_event: Channel<voice_decode::DownloadEvent>,
) -> Result<String, String> {
    let curr_time;
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(data) => {
            curr_time = data;
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }

    // get app data dir
    let app_local_data_dir_path;
    let app_local_data_dir = app_handle.path().app_local_data_dir();
    // let app_local_data_dir = tauri::path::PathResolver::app_local_data_dir(&());
    match app_local_data_dir {
        Ok(path_buf) => {
            app_local_data_dir_path = path_buf.to_str().unwrap().to_string();
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }
    let pcm_file_name = format!("koe_{}.pcm", curr_time.as_millis());
    let mp3_file_name = format!("koe_{}.mp3", curr_time.as_millis());

    let pcm_file_path = format!("{}/{}", app_local_data_dir_path, pcm_file_name);
    let mp3_file_path = format!("{}/{}", app_local_data_dir_path, mp3_file_name);
    match voice_decode::silk_v3_decoder(path, &pcm_file_path, on_event) {
        Ok(_) => {
            // Ok(out_file_path)
            match ffmpeg::convert_pcm_to_mp3(&pcm_file_path, &mp3_file_path) {
                Ok(_) => {
                    // delete pcm file
                    match std::fs::remove_file(&pcm_file_path) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(e.to_string());
                        }
                    }
                    Ok(mp3_file_name)
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn delete_mp3_file(app_handle: AppHandle, file_name: String) -> Result<(), String> {
    let app_local_data_dir = app_handle.path().app_local_data_dir();
    let mp3_file_path = format!(
        "{}/{}",
        app_local_data_dir.unwrap().to_str().unwrap(),
        file_name
    );
    match fs::remove_file(mp3_file_path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to delete file: {}", e)),
    }
}
#[tauri::command]
fn save_mp3_file(app_handle: AppHandle,file_name:&str,out_path:&str) -> Result<(), String> {
    let app_local_data_dir = app_handle.path().app_local_data_dir();
    let mp3_file_path = format!(
        "{}/{}",
        app_local_data_dir.unwrap().to_str().unwrap(),
        file_name
    );
    // copy file
    match fs::copy(mp3_file_path, out_path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to copy file: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![tauri_plugin_sql::Migration {
        version: 1,
        description: "create_initial_tables",
        sql: "CREATE TABLE history (\
            id INTEGER PRIMARY KEY, \
            src_path TEXT, \
            src_file_name TEXT,\
            out_file_name TEXT,\
            status TEXT,\
            crate_at INTEGER);",
        kind: tauri_plugin_sql::MigrationKind::Up,
    }];

    tauri::Builder::default()
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:koe.db", migrations)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, read_file, delete_mp3_file,save_mp3_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
