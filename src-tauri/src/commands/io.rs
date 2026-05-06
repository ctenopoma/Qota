use tauri::State;
use tauri_plugin_dialog::{DialogExt, FilePath};
use crate::error::{Error, Result};
use crate::models::AppData;
use crate::store::AppState;

#[tauri::command]
pub async fn load_all(state: State<'_, AppState>) -> Result<AppData> {
    Ok(state.data.lock().clone())
}

#[tauri::command]
pub async fn export_json(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String> {
    // ダイアログは非同期 (channel 経由で結果を受け取る)
    let path: Option<FilePath> = app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name("qota-export.json")
        .blocking_save_file();
    let Some(path) = path else { return Err(Error::Cancelled); };
    let path_buf = path.into_path().map_err(|e| Error::InvalidInput(e.to_string()))?;
    let data = state.data.lock().clone();
    let json = serde_json::to_vec_pretty(&data)?;
    std::fs::write(&path_buf, json)?;
    Ok(path_buf.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn import_json(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<AppData> {
    let path: Option<FilePath> = app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .blocking_pick_file();
    let Some(path) = path else { return Err(Error::Cancelled); };
    let path_buf = path.into_path().map_err(|e| Error::InvalidInput(e.to_string()))?;
    let bytes = std::fs::read(&path_buf)?;
    let new_data: AppData = serde_json::from_slice(&bytes)?;
    if new_data.schema_version != 1 {
        return Err(Error::UnsupportedSchema(new_data.schema_version));
    }
    {
        let mut data = state.data.lock();
        *data = new_data.clone();
    }
    state.save()?;
    Ok(new_data)
}

#[tauri::command]
pub async fn reset_sample_data(state: State<'_, AppState>) -> Result<AppData> {
    let seed = crate::seed::sample_data();
    {
        let mut data = state.data.lock();
        *data = seed.clone();
    }
    state.save()?;
    Ok(seed)
}
