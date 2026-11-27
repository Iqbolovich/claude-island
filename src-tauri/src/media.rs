//! Музыка через Windows SMTC — работает со Spotify, Яндекс Музыкой, браузером
//! и вообще со всем, что показывает карточку трека в системе.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

#[derive(Serialize, Clone, Default, PartialEq)]
pub struct Media {
    pub has: bool,
    pub playing: bool,
    pub title: String,
    pub artist: String,
    /// секунды
    pub pos: f64,
    pub dur: f64,
    /// номер обложки: меняется вместе с картинкой, саму картинку шлём отдельно (media-cover)
    pub cover_id: u32,
}

fn snapshot(session: &Session) -> windows::core::Result<Media> {
    let props = session.TryGetMediaPropertiesAsync()?.get()?;
    let status = session.GetPlaybackInfo()?.PlaybackStatus()?;
    let tl = session.GetTimelineProperties()?;

    let ticks = 1e7;
    let start = tl.StartTime()?.Duration as f64 / ticks;
    let end = tl.EndTime()?.Duration as f64 / ticks;
    let pos = tl.Position()?.Duration as f64 / ticks;

    let mut artist = props.Artist()?.to_string();
    if artist.is_empty() {
        artist = props.AlbumArtist().map(|a| a.to_string()).unwrap_or_default();
    }

    Ok(Media {
        has: true,
        playing: status == Status::Playing,
        title: props.Title()?.to_string(),
        artist,
        pos: (pos - start).max(0.0),
        dur: (end - start).max(0.0),
        cover_id: 0,
    })
}

/// Обложка тяжёлая, поэтому читается только при смене трека.
fn read_cover(session: &Session) -> Option<String> {
    let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 4_000_000 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size).ok()?.get().ok()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(&buf)))
}

/// Текущая обложка (номер, data-URL). Картинка весит десятки–сотни КБ, поэтому в окно она
/// уходит только при смене трека, а не в каждом обновлении раз в секунду, как раньше.
static COVER: Mutex<(u32, String)> = Mutex::new((0, String::new()));

#[derive(Serialize, Clone)]
pub struct Cover { pub id: u32, pub cover: String }

pub fn current_cover() -> Cover {
    let c = COVER.lock().unwrap();
    Cover { id: c.0, cover: c.1.clone() }
}

fn read_once(app: &AppHandle, mgr: &mut Option<Manager>, cover_key: &mut String) -> Media {
    // менеджер SMTC создаём один раз (раньше — каждую секунду, с двумя блокирующими WinRT-вызовами)
    if mgr.is_none() {
        *mgr = Manager::RequestAsync().and_then(|op| op.get()).ok();
    }
    let Some(m_) = mgr.as_ref() else { return Media::default() };
    let Ok(session) = m_.GetCurrentSession() else {
        return Media::default();
    };
    let Ok(mut m) = snapshot(&session) else {
        return Media::default();
    };
    let key = format!("{}|{}", m.title, m.artist);
    if key != *cover_key {
        *cover_key = key;
        let img = read_cover(&session).unwrap_or_default();
        let cov = {
            let mut c = COVER.lock().unwrap();
            c.0 = c.0.wrapping_add(1);
            c.1 = img;
            Cover { id: c.0, cover: c.1.clone() }
        };
        let _ = app.emit("media-cover", &cov);
    }
    m.cover_id = COVER.lock().unwrap().0;
    m
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut last = Media::default();
        let mut key = String::new();
        let mut mgr: Option<Manager> = None;
        let mut tick = 0u32;
        loop {
            tick += 1;
            let m = read_once(&app, &mut mgr, &mut key);
            // менеджер мог «умереть» (перезапуск служб) — раз в минуту берём заново
            if tick % 60 == 0 { mgr = None; }
            // повтор раз в 5 с: первое событие могло уйти до загрузки интерфейса
            if m != last || tick % 5 == 0 {
                let _ = app.emit("media-update", &m);
                last = m;
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    });
}

pub fn control(action: String) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let Ok(mgr) = Manager::RequestAsync().and_then(|op| op.get()) else {
            return;
        };
        let Ok(s) = mgr.GetCurrentSession() else {
            return;
        };
        if action == "focus" {
            let aumid = s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default();
            let title = s
                .TryGetMediaPropertiesAsync()
                .and_then(|o| o.get())
                .and_then(|p| p.Title())
                .map(|h| h.to_string())
                .unwrap_or_default();
            focus_source(&aumid, &title);
            return;
        }
        if let Some(sec) = action.strip_prefix("seek:").and_then(|v| v.parse::<f64>().ok()) {
            let _ = s.TryChangePlaybackPositionAsync((sec * 1e7) as i64).and_then(|o| o.get());
            return;
        }
        let _ = match action.as_str() {
            "next" => s.TrySkipNextAsync().and_then(|o| o.get()),
            "prev" => s.TrySkipPreviousAsync().and_then(|o| o.get()),
            _ => s.TryTogglePlayPauseAsync().and_then(|o| o.get()),
        };
    });
}

/* ---------- клик по треку: вывести вперёд программу, которая играет ---------- */

struct Found {
    aumid: String,
    title: String,
    me: u32,
    best: Option<(u8, windows::Win32::Foundation::HWND)>,
}

/// Имя exe процесса без расширения, в нижнем регистре.
fn exe_stem(pid: u32) -> String {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(h);
        if !ok { return String::new(); }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let file = path.rsplit('\\').next().unwrap_or_default().to_lowercase();
        file.trim_end_matches(".exe").to_string()
    }
}

unsafe extern "system" fn enum_cb(hwnd: windows::Win32::Foundation::HWND, lp: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible};
    let f = &mut *(lp.0 as *mut Found);
    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }
    let mut buf = [0u16; 512];
    let n = GetWindowTextW(hwnd, &mut buf);
    if n <= 0 {
        return true.into();
    }
    let wtitle = String::from_utf16_lossy(&buf[..n as usize]).to_lowercase();
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == f.me {
        return true.into();
    }
    let stem = exe_stem(pid);
    // программа совпала: AUMID «Chrome», «MSEdge», «Spotify.exe» содержит имя exe
    let by_app = !stem.is_empty() && f.aumid.contains(&stem);
    // в заголовке окна название трека (вкладка YouTube, Яндекс Музыка и т. п.)
    let by_title = f.title.chars().count() >= 3 && wtitle.contains(&f.title);
    let score = by_app as u8 + 2 * by_title as u8;
    if score > 0 && f.best.map_or(true, |(b, _)| score > b) {
        f.best = Some((score, hwnd));
    }
    true.into()
}

/// Находит окно источника звука и выводит его на передний план.
/// Если окно не нашлось, а источник — приложение из Магазина, запускает его по AUMID.
fn focus_source(aumid: &str, title: &str) {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_MENU};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW};
    let mut f = Found {
        aumid: aumid.to_lowercase(),
        title: title.to_lowercase(),
        me: std::process::id(),
        best: None,
    };
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut f as *mut Found as isize));
        if let Some((_, hwnd)) = f.best {
            let _ = ShowWindow(hwnd, if IsIconic(hwnd).as_bool() { SW_RESTORE } else { SW_SHOW });
            // Windows не даёт фоновому окну забирать фокус; короткое нажатие Alt снимает запрет
            keybd_event(VK_MENU.0 as u8, 0, Default::default(), 0);
            keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);
            let _ = SetForegroundWindow(hwnd);
            return;
        }
        if aumid.contains('!') {
            use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_LOCAL_SERVER};
            use windows::Win32::UI::Shell::{ApplicationActivationManager, IApplicationActivationManager, AO_NONE};
            if let Ok(m) = CoCreateInstance::<_, IApplicationActivationManager>(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER) {
                let _ = m.ActivateApplication(&windows::core::HSTRING::from(aumid), &windows::core::HSTRING::new(), AO_NONE);
            }
        }
    }
}
