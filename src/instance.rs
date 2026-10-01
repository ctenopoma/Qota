// 二重起動の防止。
// 起動したプロセスごとに qota.json を丸ごと上書きするので、2 つ開くと後から保存した方が前の変更を消してしまう。
// データフォルダの qota.lock を「ほかのプロセスは読むだけ」の共有モードで開いたままにし、
// 開けなかった 2 つ目は、すでに起動しているウィンドウを前に出して終わる。
// ロックはデータフォルダ単位なので、QOTA_DATA_DIR で別のデータを指す起動とはぶつからない。

use std::fs::File;
use std::path::Path;
use std::time::Duration;

/// ロックを取る。ほかで起動中なら既存のウィンドウを前に出して None を返す。
/// 相手が起動した直後でウィンドウがまだ無いこともあるので、200ms ごとに約 3 秒くり返す。
/// 返した File は main の最後まで持っておくこと（drop するとロックが外れる）
pub fn lock_or_activate(dir: &Path) -> std::io::Result<Option<File>> {
    for _ in 0..15 {
        if let Some(f) = try_lock(dir)? {
            return Ok(Some(f));
        }
        if activate_existing(dir) {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    rfd::MessageDialog::new()
        .set_title("Qota")
        .set_description("Qota はすでに起動しています")
        .show();
    Ok(None)
}

#[cfg(windows)]
fn try_lock(dir: &Path) -> std::io::Result<Option<File>> {
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .share_mode(FILE_SHARE_READ)
        .open(dir.join("qota.lock"))
    {
        Ok(mut f) => {
            // 2 つ目のプロセスが前に出すウィンドウを探せるよう、自分の PID を書いておく
            write!(f, "{}", std::process::id())?;
            f.flush()?;
            Ok(Some(f))
        }
        Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
fn try_lock(dir: &Path) -> std::io::Result<Option<File>> {
    File::create(dir.join("qota.lock")).map(Some)
}

/// qota.lock の PID のウィンドウを前に出す。最小化されているときだけ元に戻す（最大化を解かないため）
#[cfg(windows)]
fn activate_existing(dir: &Path) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, SetForegroundWindow, ShowWindowAsync, SW_RESTORE};
    let Ok(text) = std::fs::read_to_string(dir.join("qota.lock")) else { return false };
    let Ok(pid) = text.trim().parse::<u32>() else { return false };
    let Some(hwnd) = find_main_window(pid) else { return false };
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindowAsync(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }
    true
}

#[cfg(not(windows))]
fn activate_existing(_dir: &Path) -> bool {
    false
}

/// PID が一致し、タイトルが "Qota" で、親を持たないトップレベルウィンドウ。
/// winit は同じプロセスにタイトルなしの補助ウィンドウも作るので、タイトルでも絞る
#[cfg(windows)]
fn find_main_window(pid: u32) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowTextW, GetWindowThreadProcessId, GW_OWNER,
    };

    struct Search {
        pid: u32,
        found: HWND,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = unsafe { &mut *(lparam as *mut Search) };
        let mut window_pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut window_pid) };
        if window_pid != search.pid || !unsafe { GetWindow(hwnd, GW_OWNER) }.is_null() {
            return 1;
        }
        let mut buf = [0u16; 16];
        let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if String::from_utf16_lossy(&buf[..n.max(0) as usize]) != "Qota" {
            return 1;
        }
        search.found = hwnd;
        0 // 見つかったので列挙を止める
    }

    let mut search = Search { pid, found: std::ptr::null_mut() };
    unsafe { EnumWindows(Some(visit), &mut search as *mut Search as LPARAM) };
    (!search.found.is_null()).then_some(search.found)
}
