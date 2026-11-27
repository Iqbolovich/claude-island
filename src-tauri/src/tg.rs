//! Telegram через системные уведомления Windows: читаем то, что Telegram Desktop
//! кладёт в центр уведомлений. Прочитал чат в Telegram — он сам убирает уведомления,
//! и бейдж на острове гаснет.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::UI::Notifications::Management::{
    UserNotificationListener as Listener, UserNotificationListenerAccessStatus as Access,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

#[derive(Serialize, Clone, PartialEq)]
pub struct Msg {
    pub from: String,
    pub text: String,
}

#[derive(Serialize, Clone, PartialEq, Default)]
pub struct State {
    /// "ok" | "denied" | "off"
    pub status: String,
    /// от старых к новым
    pub items: Vec<Msg>,
    /// новые сообщения по заголовку окна Telegram — запасной источник, если уведомления
    /// Windows у человека не работают. Это прирост с момента «прочитано», а не всё число
    /// в заголовке: там бывают сотни тысяч из каналов без звука.
    pub count: u32,
}

/// Для диагностики в настройках.
#[derive(Serialize, Clone, Default)]
pub struct Diag {
    /// allowed | denied | unspecified
    pub access: String,
    pub running: bool,
    pub title_count: Option<u32>,
    /// уведомления Telegram хоть раз попадали в центр уведомлений Windows
    pub seen_toasts: bool,
}

static SEEN_TG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// клик «прочитано» — заново отсчитывать прирост от текущего числа в заголовке
static RESET_BASE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Число в конце заголовка: «Telegram (5)», «Имя | Аккаунт (203335)» → 5 / 203335.
fn trailing_count(title: &str) -> u32 {
    let t = title.trim_end();
    let Some(inner) = t.strip_suffix(')') else { return 0 };
    let Some((_, num)) = inner.rsplit_once('(') else { return 0 };
    let num = num.trim();
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) { return 0; }
    num.parse().unwrap_or(0)
}

/// Ищет окна Telegram Desktop (по процессу Telegram.exe — у версии из Microsoft Store
/// в заголовке имя аккаунта, а не «Telegram») и достаёт число непрочитанных из заголовка.
/// None — Telegram не запущен; Some(0) — запущен, непрочитанных нет.
pub fn title_unread() -> Option<u32> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, CloseHandle};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId};
    unsafe extern "system" fn each(h: HWND, lp: LPARAM) -> BOOL {
        let out = &mut *(lp.0 as *mut Option<u32>);
        let mut buf = [0u16; 128];
        let n = GetWindowTextW(h, &mut buf);
        if n <= 0 { return true.into(); }
        let title = String::from_utf16_lossy(&buf[..n as usize]);
        // окно должно принадлежать Telegram.exe, а не вкладке браузера «Telegram Web»
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        let Ok(ph) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return true.into() };
        let mut path = [0u16; 512];
        let mut len = path.len() as u32;
        let ok = QueryFullProcessImageNameW(ph, PROCESS_NAME_WIN32, windows::core::PWSTR(path.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(ph);
        if !ok || !String::from_utf16_lossy(&path[..len as usize]).to_lowercase().ends_with("telegram.exe") {
            return true.into();
        }
        let count = trailing_count(&title);
        *out = Some(out.unwrap_or(0).max(count));
        true.into()
    }
    let mut res: Option<u32> = None;
    unsafe { let _ = EnumWindows(Some(each), LPARAM(&mut res as *mut _ as isize)); }
    res
}

pub fn diag() -> Diag {
    let access = Listener::Current()
        .and_then(|l| l.GetAccessStatus())
        .map(|a| match a { Access::Allowed => "allowed", Access::Denied => "denied", _ => "unspecified" })
        .unwrap_or("unspecified")
        .to_string();
    let t = title_unread();
    Diag { access, running: t.is_some(), title_count: t, seen_toasts: SEEN_TG.load(std::sync::atomic::Ordering::Relaxed) }
}

// id уведомлений Telegram, которые сейчас в центре уведомлений — чтобы убрать их по клику
thread_local! { static DBG: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new()); }
// Разобранные уведомления по id: имя приложения и текст читаются один раз, а не каждые 1,5 с
// для всех уведомлений всех приложений в центре уведомлений (их за день набираются десятки).
thread_local! { static CACHE: std::cell::RefCell<HashMap<u32, Option<Msg>>> = std::cell::RefCell::new(HashMap::new()); }
static IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn read(l: &Listener) -> windows::core::Result<Vec<(u32, Msg)>> {
    let list = l.GetNotificationsAsync(NotificationKinds::Toast)?.get()?;
    let generic = KnownNotificationBindings::ToastGeneric()?;
    let mut out = Vec::new();
    let mut present = HashSet::new();
    for n in list {
        let id = n.Id()?;
        present.insert(id);
        if let Some(cached) = CACHE.with(|c| c.borrow().get(&id).cloned()) {
            if let Some(m) = cached { out.push((id, m)); }
            continue;
        }
        let name = n
            .AppInfo()
            .and_then(|a| a.DisplayInfo())
            .and_then(|d| d.DisplayName())
            .map(|s| s.to_string())
            .unwrap_or_default();
        DBG.with(|d| { if d.borrow_mut().insert(name.clone()) { crate::ilog!("[tg] приложение в уведомлениях: {name}"); } });
        if !name.to_lowercase().contains("telegram") {
            CACHE.with(|c| c.borrow_mut().insert(id, None));
            continue;
        }
        let Ok(b) = n.Notification().and_then(|x| x.Visual()).and_then(|v| v.GetBinding(&generic)) else {
            CACHE.with(|c| c.borrow_mut().insert(id, None));
            continue;
        };
        let texts: Vec<String> = b
            .GetTextElements()
            .map(|t| t.into_iter().filter_map(|x| x.Text().ok()).map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let from = texts.first().cloned().unwrap_or_else(|| "Telegram".into());
        let text = texts.get(1..).map(|t| t.join(" ")).unwrap_or_default();
        let m = Msg { from, text };
        SEEN_TG.store(true, std::sync::atomic::Ordering::Relaxed);
        CACHE.with(|c| c.borrow_mut().insert(id, Some(m.clone())));
        out.push((id, m));
    }
    // ушедшие из центра уведомлений забываем
    CACHE.with(|c| c.borrow_mut().retain(|id, _| present.contains(id)));
    // CreationTime у всех есть, но порядок выдачи и так хронологический
    Ok(out)
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let Ok(l) = Listener::Current() else { return };
        let mut access = l.GetAccessStatus().unwrap_or(Access::Unspecified);
        if access == Access::Unspecified {
            // Windows один раз спросит разрешение на доступ к уведомлениям
            access = l.RequestAccessAsync().and_then(|op| op.get()).unwrap_or(Access::Denied);
        }
        crate::ilog!("[tg] доступ к уведомлениям: {:?}", access.0);
        let mut last = State::default();
        let mut seen: HashSet<u32> = HashSet::new();
        let mut first = true;
        let mut last_err = String::new();
        let mut last_count = 0u32;
        let mut base: Option<u32> = None;
        let mut prev_raw: Option<u32> = None;
        let mut last_fallback: Option<std::time::Instant> = None;
        let mut tick = 0u32;
        loop {
            tick += 1;
            let mut toast_now = false;
            let on = app.state::<crate::AppState>().settings.lock().unwrap().show_telegram;
            let mut st = State { status: "off".into(), items: vec![], count: 0 };
            if on {
                match title_unread() {
                    Some(raw) => {
                        // отсчёт от числа при запуске / после «прочитано»; прочитал в Telegram — число
                        // в заголовке упало, опускаем и точку отсчёта
                        if RESET_BASE.swap(false, std::sync::atomic::Ordering::Relaxed) { base = Some(raw); }
                        // скачок за 1,5 с больше чем на 50 — это не новые сообщения, а заголовок
                        // без числа (Telegram запускается, окно перерисовывается) вернулся к полному
                        // числу: сдвигаем точку отсчёта вместе с ним
                        if let (Some(b), Some(p)) = (base, prev_raw) {
                            if raw > p + 50 { base = Some(b + (raw - p)); }
                        }
                        let b = base.map_or(raw, |b| b.min(raw));
                        base = Some(b);
                        prev_raw = Some(raw);
                        // уведомления Windows работают — тексты берём оттуда, счётчик заголовка не нужен
                        if !SEEN_TG.load(std::sync::atomic::Ordering::Relaxed) { st.count = raw - b; }
                    }
                    // Telegram закрыт — при следующем запуске отсчёт заново
                    None => { base = None; prev_raw = None; }
                }
            }
            if on && access == Access::Allowed {
                st.status = "ok".into();
                let got = match read(&l) {
                    Ok(g) => { last_err.clear(); g }
                    Err(e) => {
                        // одна и та же ошибка пишется в журнал один раз, а не каждые 1,5 с
                        let s = e.to_string();
                        if s != last_err { crate::ilog!("[tg] ошибка чтения: {s}"); last_err = s; }
                        vec![]
                    }
                };
                *IDS.lock().unwrap() = got.iter().map(|(id, _)| *id).collect();
                for (id, m) in &got {
                    // при запуске старые уведомления не «выстреливают»
                    if seen.insert(*id) && !first {
                        let _ = app.emit("tg-notification", m);
                        toast_now = true;
                    }
                }
                // помним только то, что ещё в центре уведомлений (раньше набор рос бесконечно)
                seen.retain(|id| got.iter().any(|(g, _)| g == id));
                st.items = got.into_iter().map(|(_, m)| m).collect();
                first = false;
            } else if on {
                st.status = "denied".into();
                access = l.GetAccessStatus().unwrap_or(access);
            }
            // уведомления Windows не пришли, а счётчик в заголовке Telegram вырос —
            // показываем хотя бы «новое сообщение» (у части людей тосты Telegram не работают)
            // (не чаще раза в минуту — иначе активные чаты засыплют остров)
            if on && st.count > last_count && !toast_now
                && last_fallback.map_or(true, |t| t.elapsed() >= Duration::from_secs(60)) {
                let _ = app.emit("tg-notification", Msg { from: "Telegram".into(), text: format!("Новых сообщений: {}", st.count) });
                last_fallback = Some(std::time::Instant::now());
            }
            last_count = st.count;
            if st != last || tick % 4 == 0 {
                let _ = app.emit("tg-state", &st);
                last = st;
            }
            std::thread::sleep(Duration::from_millis(1500));
        }
    });
}

/// Клик по плитке: убираем уведомления Telegram из центра уведомлений.
pub fn clear() {
    RESET_BASE.store(true, std::sync::atomic::Ordering::Relaxed);
    std::thread::spawn(|| {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let Ok(l) = Listener::Current() else { return };
        for id in IDS.lock().unwrap().drain(..) {
            let _ = l.RemoveNotification(id);
        }
    });
}
