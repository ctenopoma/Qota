use crate::error::{Error, Result};
use crate::models::{Settings, SettingsPatch};
use crate::store::AppState;

pub fn update_settings(state: &AppState, patch: SettingsPatch) -> Result<Settings> {
    let updated = {
        let mut data = state.data.lock();
        if let Some(v) = patch.mode { data.settings.mode = v; }
        if let Some(v) = patch.accent { data.settings.accent = v; }
        if let Some(v) = patch.density { data.settings.density = v; }
        if let Some(v) = patch.row_height { data.settings.row_height = v; }
        if let Some(v) = patch.week_start { data.settings.week_start = v; }
        // タイムチャートの時間軸（時）。始まり < 終わり で、1 時間以上あけ、0〜24 に収める
        let start = patch.day_start_hour.unwrap_or(data.settings.day_start_hour);
        let end = patch.day_end_hour.unwrap_or(data.settings.day_end_hour);
        if patch.day_start_hour.is_some() || patch.day_end_hour.is_some() {
            if start >= end || end > 24 {
                return Err(Error::InvalidInput("day range".into()));
            }
            data.settings.day_start_hour = start;
            data.settings.day_end_hour = end;
        }

        data.settings.clone()
    };
    state.save()?;
    Ok(updated)
}
