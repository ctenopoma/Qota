// ウィンドウの大きさと最大化状態を、データと同じフォルダの window.json に保存する。
// qota.json は Tauri 版とスキーマを共有しているので、そちらには入れない。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::review::Unit;
use crate::view::SortKey;

/// 大きさはクライアント領域の物理 px（起動前は拡大率が分からないため）
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// タスクバーを除いた画面の領域（物理 px）と拡大率
#[derive(Clone, Copy, Debug)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f32,
}

fn path(dir: &Path) -> PathBuf {
    dir.join("window.json")
}

/// サイドバーと「完了済み」の開閉、今日の一覧の並び順。ウィンドウの大きさとは別のファイルにして、
/// 片方が読めなくてももう片方を失わないようにする。起動時の画面はいつも「今日」なので持たない
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct UiPrefs {
    pub sidebar_open: bool,
    pub archive_open: bool,
    pub today_sort: SortKey,
    /// 初回の操作ヒントを閉じたか
    pub hint_dismissed: bool,
    /// 今日をタイムチャートで見ているか
    pub today_chart: bool,
    /// 振り返りの期間の単位（日・週・月）
    pub review_unit: Unit,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            archive_open: false,
            today_sort: SortKey::Due,
            hint_dismissed: false,
            today_chart: false,
            review_unit: Unit::Week,
        }
    }
}

pub fn load_prefs(dir: &Path) -> UiPrefs {
    std::fs::read(dir.join("ui-state.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_prefs(dir: &Path, prefs: &UiPrefs) -> std::io::Result<()> {
    std::fs::write(dir.join("ui-state.json"), serde_json::to_vec_pretty(prefs)?)
}

pub fn load(dir: &Path) -> Option<WindowState> {
    serde_json::from_slice(&std::fs::read(path(dir)).ok()?).ok()
}

pub fn save(dir: &Path, state: &WindowState) -> std::io::Result<()> {
    std::fs::write(path(dir), serde_json::to_vec_pretty(state)?)
}

/// winit より先にプロセスを Per-Monitor DPI 対応にしておく。
/// こうしないと、起動前に読む作業領域が拡大率で縮めた値になってしまう
#[cfg(windows)]
pub fn make_dpi_aware() {
    use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
    // すでに設定済みなら失敗するだけなので戻り値は見ない
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

#[cfg(windows)]
pub fn work_area() -> Option<WorkArea> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0) };
    if ok == 0 {
        return None;
    }
    let scale = unsafe { GetDpiForSystem() } as f32 / 96.0;
    Some(WorkArea { x: r.left, y: r.top, width: r.right - r.left, height: r.bottom - r.top, scale })
}

/// ウィンドウの最小の大きさ（論理 px）。ui/app.slint の min-width / min-height と揃える
const MIN_WIDTH: f32 = 360.0;
const MIN_HEIGHT: f32 = 420.0;

/// 起動時の大きさと位置 (width, height, x, y)。
/// 前回の大きさ（無ければ論理 1280×820）を作業領域に収まるよう縮め、中央に置く
pub fn initial_geometry(saved: Option<WindowState>, area: WorkArea) -> (u32, u32, i32, i32) {
    let s = area.scale;
    let px = |logical: f32| (logical * s).round() as i32;
    let (w, h) = match saved {
        Some(st) => (st.width as i32, st.height as i32),
        None => (px(1280.0), px(820.0)),
    };
    // set_size はクライアント領域の大きさなので、タイトルバーと枠の分を残す
    let (frame_w, frame_h) = (px(16.0), px(40.0));
    let w = w.min(area.width - frame_w).max(px(MIN_WIDTH));
    let h = h.min(area.height - frame_h).max(px(MIN_HEIGHT));
    let x = area.x + ((area.width - w - frame_w) / 2).max(0);
    let y = area.y + ((area.height - h - frame_h) / 2).max(0);
    (w as u32, h as u32, x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 200% 表示で作業領域 2880×1704 の画面
    const AREA: WorkArea = WorkArea { x: 0, y: 0, width: 2880, height: 1704, scale: 2.0 };

    #[test]
    fn default_size_is_shrunk_to_fit_work_area() {
        let (w, h, x, y) = initial_geometry(None, AREA);
        assert_eq!(w, 2560);
        assert_eq!(h, 1704 - 80); // 1640 では下端がはみ出す
        assert!(x >= 0 && y >= 0);
        assert!(y + h as i32 + 80 <= 1704);
    }

    #[test]
    fn saved_size_is_kept_when_it_fits() {
        let saved = WindowState { width: 2000, height: 1200, maximized: false };
        let (w, h, x, y) = initial_geometry(Some(saved), AREA);
        assert_eq!((w, h), (2000, 1200));
        assert_eq!((x, y), ((2880 - 2000 - 32) / 2, (1704 - 1200 - 80) / 2));
    }

    #[test]
    fn prefs_round_trip_and_tolerate_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_prefs(dir.path()), UiPrefs::default());
        let p = UiPrefs {
            sidebar_open: false,
            archive_open: true,
            today_sort: SortKey::Priority,
            hint_dismissed: true,
            today_chart: true,
            review_unit: Unit::Month,
        };
        save_prefs(dir.path(), &p).unwrap();
        assert_eq!(load_prefs(dir.path()), p);
        // 古い・欠けたファイルでも既定値で補う
        std::fs::write(dir.path().join("ui-state.json"), br#"{ "archiveOpen": 1, "archive_open": true }"#).unwrap();
        assert_eq!(load_prefs(dir.path()), UiPrefs { archive_open: true, ..UiPrefs::default() });
    }

    #[test]
    fn never_smaller_than_min_size() {
        let saved = WindowState { width: 100, height: 100, maximized: false };
        let (w, h, _, _) = initial_geometry(Some(saved), AREA);
        assert_eq!((w, h), (720, 840)); // 論理 360×420 の 200%
    }
}
