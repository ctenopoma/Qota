// tauri-plugin-dialog の代わりに rfd でファイルダイアログを出す。
use std::path::{Path, PathBuf};
use crate::error::{Error, Result};
use crate::models::AppData;
use crate::store::AppState;

pub fn export_json(state: &AppState) -> Result<String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name("qota-export.json")
        .save_file()
    else {
        return Err(Error::Cancelled);
    };
    let data = state.data.lock().clone();
    let json = serde_json::to_vec_pretty(&data)?;
    std::fs::write(&path, json)?;
    Ok(path.to_string_lossy().into_owned())
}

/// 読み込むファイルを選ぶ。start_dir を渡すとそのフォルダで開く（バックアップから戻すとき）
pub fn pick_import_file(start_dir: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().add_filter("JSON", &["json"]);
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file()
}

/// 選んだファイルで今のデータを置き換える。形式が違えば何も変えない
pub fn import_from(state: &AppState, path: &Path) -> Result<()> {
    let bytes = std::fs::read(path)?;
    let new_data: AppData = serde_json::from_slice(&bytes)?;
    if new_data.schema_version != 1 {
        return Err(Error::UnsupportedSchema(new_data.schema_version));
    }
    *state.data.lock() = new_data;
    state.save()
}

pub fn reset_sample_data(state: &AppState) -> Result<()> {
    *state.data.lock() = crate::seed::sample_data();
    state.save()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn import_rejects_broken_file_without_touching_data() {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        let before = s.data.lock().clone();
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, b"{ not json").unwrap();
        assert!(import_from(&s, &bad).is_err());
        assert_eq!(*s.data.lock(), before);
    }
}
