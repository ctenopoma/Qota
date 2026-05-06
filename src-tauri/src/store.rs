use std::path::PathBuf;
use parking_lot::Mutex;
use crate::error::Result;
use crate::models::AppData;

pub struct AppState {
    pub data: Mutex<AppData>,
    pub path: PathBuf,
}

impl AppState {
    pub fn load_or_init(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir)?;
        let path = app_data_dir.join("qota.json");
        let data = if path.exists() {
            let bytes = std::fs::read(&path)?;
            let data: AppData = serde_json::from_slice(&bytes)?;
            // migrate(&mut data)?;
            data
        } else {
            let seed = crate::seed::sample_data();
            atomic_write(&path, &seed)?;
            seed
        };
        Ok(Self { data: Mutex::new(data), path })
    }

    pub fn save(&self) -> Result<()> {
        let guard = self.data.lock();
        atomic_write(&self.path, &*guard)
    }
}

fn atomic_write(path: &PathBuf, data: &AppData) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(data)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
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
}
