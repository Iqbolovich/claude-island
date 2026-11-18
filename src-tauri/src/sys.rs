//! Мелкие системные события: зарядка и батарея, буфер обмена, раскладка,
//! камера/микрофон, полноэкранные приложения.

use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Globalization::LCIDToLocaleName;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Serialize, Clone)]
struct Power { charging: bool, pct: u32, low: bool }

fn power() -> Option<(bool, u32)> {
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).ok()? };
    // 128 — батареи нет (настольный ПК), 255 — неизвестно
    if s.BatteryFlag == 128 || s.BatteryFlag == 255 || s.BatteryLifePercent > 100 { return None; }
    Some((s.ACLineStatus == 1, s.BatteryLifePercent as u32))
}

/// Язык раскладки активного окна: «RU», «EN»…
fn layout() -> (isize, String) {
    unsafe {
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, None);
        let hkl = GetKeyboardLayout(tid);
        let lcid = (hkl.0 as usize & 0xffff) as u32;
        let mut buf = [0u16; 85];
        let n = LCIDToLocaleName(lcid, Some(&mut buf), 0);
        let name = String::from_utf16_lossy(&buf[..(n.max(1) - 1) as usize]);
        (fg.0 as isize, name.split('-').next().unwrap_or("").to_uppercase())
    }
}

#[derive(Serialize, Clone, PartialEq, Default)]
struct Privacy { cam: bool, mic: bool, app: String }

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }

unsafe fn open(root: HKEY, path: &str) -> Option<HKEY> {
    let mut k = HKEY::default();
    let p = wide(path);
    RegOpenKeyExW(root, PCWSTR(p.as_ptr()), 0, KEY_READ, &mut k).is_ok().then_some(k)
}

unsafe fn qword(k: HKEY, name: &str) -> u64 {
    let (mut v, mut sz) = (0u64, 8u32);
    let n = wide(name);
    let _ = RegQueryValueExW(k, PCWSTR(n.as_ptr()), None, None, Some(&mut v as *mut u64 as *mut u8), Some(&mut sz));
    v
}

/// Ищет в ConsentStore приложение, которое прямо сейчас использует камеру/микрофон.
unsafe fn in_use(k: HKEY, depth: u8) -> Option<String> {
    let mut i = 0;
    loop {
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        if RegEnumKeyExW(k, i, PWSTR(buf.as_mut_ptr()), &mut len, None, PWSTR::null(), None, None).is_err() { return None; }
        i += 1;
        let name = String::from_utf16_lossy(&buf[..len as usize]);
        let Some(sub) = open(k, &name) else { continue };
        let found = if name == "NonPackaged" && depth == 0 {
            in_use(sub, 1)
        } else if qword(sub, "LastUsedTimeStart") > 0 && qword(sub, "LastUsedTimeStop") == 0 {
            // «C:#Program Files#Zoom#Zoom.exe» → Zoom; «Microsoft.WindowsCamera_8wek…» → WindowsCamera
            let short = name.rsplit('#').next().unwrap_or(&name).trim_end_matches(".exe");
            let short = short.split('_').next().unwrap_or(short);
            Some(short.rsplit('.').next().unwrap_or(short).to_string())
        } else { None };
        let _ = RegCloseKey(sub);
        if found.is_some() { return found; }
    }
}

fn privacy() -> Privacy {
    let base = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\";
    let mut p = Privacy::default();
    for (cap, cam) in [("webcam", true), ("microphone", false)] {
        unsafe {
            if let Some(k) = open(HKEY_CURRENT_USER, &format!("{base}{cap}")) {
                if let Some(app) = in_use(k, 0) {
                    if cam { p.cam = true } else { p.mic = true }
                    if p.app.is_empty() { p.app = app; }
                }
                let _ = RegCloseKey(k);
            }
        }
    }
    p
}

fn fullscreen() -> bool {
    unsafe { SHQueryUserNotificationState() }
        .is_ok_and(|s| s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE)
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut clip = unsafe { GetClipboardSequenceNumber() };
        let mut pw = power();
        let mut warned = false;
        let mut tick = 0u32;
        let (mut lay_win, mut lay) = layout();
        let mut priv_last = Privacy::default();
        let mut fs_hidden = false;
        loop {
            let (w, l) = layout();
            if w == lay_win && l != lay && !l.is_empty() {
                let _ = app.emit("layout", l.clone());
            }
            (lay_win, lay) = (w, l);
            if tick % 5 == 0 {
                let p = privacy();
                // повтор раз в ~6 с: первое событие могло уйти до загрузки интерфейса
                if p != priv_last || tick % 20 == 0 { let _ = app.emit("privacy", p.clone()); }
                priv_last = p;
            }
            if tick % 3 == 0 {
                let want = app.state::<crate::AppState>().settings.lock().unwrap().hide_fullscreen;
                let fs = want && fullscreen();
                if fs != fs_hidden {
                    fs_hidden = fs;
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = if fs { win.hide() } else { win.show() };
                    }
                }
            }
            let c = unsafe { GetClipboardSequenceNumber() };
            if c != clip {
                clip = c;
                let _ = app.emit("copied", ());
            }
            if tick % 5 == 0 {
                let now = power();
                if let (Some((was, _)), Some((ac, pct))) = (pw, now) {
                    if ac != was {
                        let _ = app.emit("power", Power { charging: ac, pct, low: false });
                    }
                    if !ac && pct <= 15 && !warned {
                        warned = true;
                        let _ = app.emit("power", Power { charging: false, pct, low: true });
                    }
                    if ac || pct > 20 { warned = false; }
                }
                pw = now;
            }
            tick = tick.wrapping_add(1);
            std::thread::sleep(Duration::from_millis(300));
        }
    });
}
