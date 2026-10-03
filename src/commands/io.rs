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

/// 読み込めないファイルの理由を、画面に出せる日本語で返す。読めるなら None
pub fn import_problem(path: &Path) -> Option<String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return Some("ファイルを開けません".into()),
    };
    match serde_json::from_slice::<AppData>(crate::store::strip_bom(&bytes)) {
        Err(_) => Some("Qota のデータの形式ではありません".into()),
        Ok(d) if d.schema_version != 1 => Some("対応していない版のデータです".into()),
        Ok(_) => None,
    }
}

/// 選んだファイルで今のデータを置き換える。形式が違えば何も変えない
pub fn import_from(state: &AppState, path: &Path) -> Result<()> {
    let bytes = std::fs::read(path)?;
    let new_data: AppData = serde_json::from_slice(crate::store::strip_bom(&bytes))?;
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

    #[test]
    fn import_reads_bom_file_and_explains_broken_one() {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        let json = serde_json::to_vec(&*s.data.lock()).unwrap();
        let bom = dir.path().join("bom.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend(json);
        std::fs::write(&bom, bytes).unwrap();
        assert!(import_problem(&bom).is_none());
        assert!(import_from(&s, &bom).is_ok());

        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, b"{ not json").unwrap();
        assert!(import_problem(&bad).is_some());
    }
}
