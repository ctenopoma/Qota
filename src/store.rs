use std::io::Write;
use std::path::{Path, PathBuf};
use parking_lot::Mutex;
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use crate::error::Result;
use crate::models::AppData;

/// 現行 Tauri 版 (identifier = com.example.qota) のデータ置き場。
pub const TAURI_IDENTIFIER: &str = "com.example.qota";
/// 試作のあいだは Tauri 版の qota.json に触れないよう、別ディレクトリを使う。
pub const PROTOTYPE_IDENTIFIER: &str = "com.example.qota.slint";

/// 毎日のバックアップを何日分残すか
const BACKUP_KEEP: usize = 7;

/// `QOTA_DATA_DIR` があればそれを、無ければ `%APPDATA%\com.example.qota.slint` を使う。
/// ここではパスを決めるだけで、書き込みは二重起動のロックを取ってから行う。
pub fn resolve_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("QOTA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .expect("failed to resolve data dir")
        .join(PROTOTYPE_IDENTIFIER)
}

/// 既定のデータフォルダのときだけ、初回に Tauri 版の qota.json をコピーしてくる（元ファイルは書き換えない）。
pub fn seed_from_tauri_if_missing(dir: &Path) -> Result<()> {
    if std::env::var_os("QOTA_DATA_DIR").is_some() {
        return Ok(());
    }
    let dest = dir.join("qota.json");
    let src = dirs::data_dir().map(|d| d.join(TAURI_IDENTIFIER).join("qota.json"));
    if let Some(src) = src.filter(|s| s.exists()) {
        if !dest.exists() {
            std::fs::copy(&src, &dest)?;
        }
    }
    Ok(())
}

pub struct AppState {
    pub data: Mutex<AppData>,
    pub path: PathBuf,
}

impl AppState {
    pub fn load_or_init(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir)?;
        let path = app_data_dir.join("qota.json");
        let mut data = if path.exists() {
            let bytes = std::fs::read(&path)?;
            let data: AppData = serde_json::from_slice(&bytes)?;
            // migrate(&mut data)?;
            data
        } else {
            let seed = crate::seed::sample_data();
            atomic_write(&path, &seed)?;
            seed
        };

        // Purge completed tasks older than 2 months
        let cutoff = Utc::now() - Duration::days(60);
        let before_len = data.tasks.len();
        data.tasks.retain(|t| match &t.completed_at {
            Some(ts) => DateTime::parse_from_rfc3339(ts)
                .map(|dt| dt.with_timezone(&Utc) >= cutoff)
                .unwrap_or(true),
            None => true,
        });
        if data.tasks.len() != before_len {
            backup_or_log(&app_data_dir);
            atomic_write(&path, &data)?;
        }

        Ok(Self { data: Mutex::new(data), path })
    }

    pub fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            backup_or_log(dir);
        }
        let guard = self.data.lock();
        atomic_write(&self.path, &*guard)
    }

    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }
}

/// 一時ファイルに書いて、ディスクへ書き出し終えてから置き換える。
/// （NTFS は名前の変更だけを記録するので、書き出しを待たないと「名前は新しいのに中身が空」が起こり得る）
pub fn atomic_write(path: &Path, data: &AppData) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(data)?;
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(&json)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

// ───── バックアップ ─────

pub fn backups_dir(dir: &Path) -> PathBuf {
    dir.join("backups")
}

/// バックアップに失敗しても本体の保存は止めない
fn backup_or_log(dir: &Path) {
    if let Err(e) = backup_if_needed(dir, Local::now().date_naive()) {
        eprintln!("backup failed: {e}");
    }
}

/// その日最初の書き込みの直前に、今の qota.json を backups\qota-YYYY-MM-DD.json に残す（＝その日の変更前の状態）。
/// 「今日の分があるか」はファイルの有無で判断する。同じ日に再起動しても朝の分を上書きしない
pub fn backup_if_needed(dir: &Path, today: NaiveDate) -> std::io::Result<Option<PathBuf>> {
    let src = dir.join("qota.json");
    if !src.exists() {
        return Ok(None);
    }
    let backups = backups_dir(dir);
    let dest = backups.join(format!("qota-{}.json", today.format("%Y-%m-%d")));
    if dest.exists() {
        return Ok(None);
    }
    copy_durably(&src, &dest)?;
    prune(&backups, is_daily_backup, BACKUP_KEEP)?;
    Ok(Some(dest))
}

/// インポート・リセットの直前に、日付に関係なく 1 つ残す（その日すでに変更していても戻せるように）
pub fn backup_before(dir: &Path, reason: &str, now: DateTime<Local>) -> std::io::Result<Option<PathBuf>> {
    let src = dir.join("qota.json");
    if !src.exists() {
        return Ok(None);
    }
    let backups = backups_dir(dir);
    let dest = backups.join(format!("qota-{}-before-{reason}.json", now.format("%Y-%m-%d-%H%M%S")));
    copy_durably(&src, &dest)?;
    prune(&backups, is_event_backup, BACKUP_KEEP)?;
    Ok(Some(dest))
}

/// いちばん新しいバックアップの日付 (YYYY-MM-DD)。設定画面に出す
pub fn latest_backup_date(dir: &Path) -> Option<String> {
    let mut names: Vec<String> = std::fs::read_dir(backups_dir(dir)).ok()?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| is_daily_backup(n) || is_event_backup(n))
        .collect();
    names.sort();
    names.last().map(|n| n[5..15].to_string())
}

/// 途中で落ちて壊れたファイルが「今日の分」と見なされないよう、一時ファイル経由で置く
fn copy_durably(src: &Path, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("json.tmp");
    std::fs::copy(src, &tmp)?;
    // Windows ではディスクへの書き出し (FlushFileBuffers) に書き込み権限が要る
    std::fs::OpenOptions::new().write(true).open(&tmp)?.sync_all()?;
    std::fs::rename(&tmp, dest)
}

/// qota-YYYY-MM-DD.json だけ
fn is_daily_backup(name: &str) -> bool {
    name.len() == "qota-2026-01-01.json".len()
        && name.starts_with("qota-")
        && name.ends_with(".json")
        && NaiveDate::parse_from_str(&name[5..15], "%Y-%m-%d").is_ok()
}

/// qota-YYYY-MM-DD-HHMMSS-before-*.json だけ
fn is_event_backup(name: &str) -> bool {
    name.starts_with("qota-")
        && name.ends_with(".json")
        && name.len() > 23
        && NaiveDate::parse_from_str(&name[5..15], "%Y-%m-%d").is_ok()
        && name[15..].starts_with('-')
        && name.contains("-before-")
}

/// 名前が日付順に並ぶので、名前で並べて古いものから消す。pred に合うファイル以外には触らない
fn prune(dir: &Path, pred: fn(&str) -> bool, keep: usize) -> std::io::Result<()> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| pred(n))
        .collect();
    names.sort();
    let excess = names.len().saturating_sub(keep);
    for n in &names[..excess] {
        std::fs::remove_file(dir.join(n))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let data = crate::seed::sample_data();

        assert!(atomic_write(&path, &data).is_ok());
        assert!(path.exists());

        // Ensure .tmp is removed
        assert!(!dir.path().join("test.json.tmp").exists());

        let read_bytes = std::fs::read(&path).unwrap();
        let read_data: AppData = serde_json::from_slice(&read_bytes).unwrap();
        assert_eq!(read_data.schema_version, data.schema_version);
    }

    #[test]
    fn legacy_time_fields_are_ignored() {
        // Tauri 版で保存された時間計測フィールド付きの JSON を読めること
        let json = r#"{
          "schemaVersion": 1,
          "tasks": [{
            "id": "a", "title": "t", "notes": "", "status": "todo", "priority": null,
            "due": null, "projectId": null, "tagIds": [],
            "subtasks": [{ "id": "s", "title": "x", "done": false,
                           "estimatedMinutes": 30, "actualMinutes": 12, "startedAt": null }],
            "estimatedMinutes": 60, "actualMinutes": null, "startedAt": "2026-05-06T09:00:00Z",
            "createdAt": "2026-05-06T09:00:00Z", "updatedAt": "2026-05-06T09:00:00Z",
            "completedAt": null, "interval": null, "recurrenceId": null, "recurrenceEnd": null
          }],
          "projects": [], "tags": [],
          "settings": { "mode": "dark", "accent": "green", "density": "comfortable",
                        "rowHeight": 12, "weekStart": "mon" }
        }"#;
        let data: AppData = serde_json::from_str(json).unwrap();
        assert_eq!(data.tasks[0].subtasks[0].title, "x");
        let out = serde_json::to_string(&data).unwrap();
        assert!(!out.contains("estimatedMinutes"));
    }

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn daily_backup_is_made_once_per_day_with_the_state_before_the_change() {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        let original = std::fs::read(&s.path).unwrap();

        let first = backup_if_needed(dir.path(), day("2026-10-01")).unwrap();
        assert!(first.is_some());
        // 同じ日の 2 回目は作らない（朝の分を上書きしない）
        std::fs::write(&s.path, b"changed").unwrap();
        assert!(backup_if_needed(dir.path(), day("2026-10-01")).unwrap().is_none());
        assert_eq!(std::fs::read(first.unwrap()).unwrap(), original);
        assert!(!backups_dir(dir.path()).join("qota-2026-10-01.json.tmp").exists());
    }

    #[test]
    fn keeps_latest_seven_and_leaves_other_files_alone() {
        let dir = tempdir().unwrap();
        AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        let b = backups_dir(dir.path());
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("memo.txt"), b"keep me").unwrap();
        std::fs::write(b.join("qota-2026-10-01-120000-before-import.json"), b"{}").unwrap();
        for d in 1..=8 {
            backup_if_needed(dir.path(), day(&format!("2026-09-{d:02}"))).unwrap();
        }
        let mut daily: Vec<String> = std::fs::read_dir(&b).unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| is_daily_backup(n))
            .collect();
        daily.sort();
        assert_eq!(daily.len(), 7);
        assert_eq!(daily[0], "qota-2026-09-02.json");
        assert!(b.join("memo.txt").exists());
        assert!(b.join("qota-2026-10-01-120000-before-import.json").exists());
        assert_eq!(latest_backup_date(dir.path()).as_deref(), Some("2026-10-01"));
    }

    #[test]
    fn backup_failure_does_not_block_saving() {
        let dir = tempdir().unwrap();
        let s = AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        // backups をファイルにしてフォルダを作れなくする
        std::fs::write(backups_dir(dir.path()), b"not a dir").unwrap();
        s.data.lock().tasks.clear();
        assert!(s.save().is_ok());
        let saved: AppData = serde_json::from_slice(&std::fs::read(&s.path).unwrap()).unwrap();
        assert!(saved.tasks.is_empty());
    }

    #[test]
    fn backup_before_import_is_made_even_if_daily_exists() {
        let dir = tempdir().unwrap();
        AppState::load_or_init(dir.path().to_path_buf()).unwrap();
        backup_if_needed(dir.path(), Local::now().date_naive()).unwrap();
        let p = backup_before(dir.path(), "import", Local::now()).unwrap().unwrap();
        assert!(p.exists());
        assert!(is_event_backup(&p.file_name().unwrap().to_string_lossy()));
    }
}
