use tauri::Manager;

pub mod commands;
pub mod error;
pub mod models;
pub mod seed;
pub mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()
                .expect("failed to resolve app_data_dir");
            let state = store::AppState::load_or_init(dir)
                .expect("failed to load app state");
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::io::load_all,
            commands::io::export_json,
            commands::io::import_json,
            commands::io::reset_sample_data,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::toggle_task,
            commands::tasks::delete_task,
            commands::subtasks::add_subtask,
            commands::subtasks::toggle_subtask,
            commands::subtasks::delete_subtask,
            commands::subtasks::update_subtask,
            commands::subtasks::move_subtask,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::tags::create_tag,
            commands::tags::delete_tag,
            commands::settings::update_settings,
            commands::search::search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
