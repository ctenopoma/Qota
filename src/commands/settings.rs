use crate::error::Result;
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

        data.settings.clone()
    };
    state.save()?;
    Ok(updated)
}
