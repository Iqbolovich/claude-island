#[macro_use]
pub mod log;
mod hooks;
#[cfg(windows)]
mod guard;
#[cfg(windows)]
mod chatwatch;
#[cfg(windows)]
mod media;
mod secret;
mod settings;
#[cfg(windows)]
mod sys;
#[cfg(windows)]
mod tg;
mod usage;
#[cfg(windows)]
mod volume;

use settings::Settings;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub config_dir: PathBuf,
    pub hidden: AtomicBool,
    /// будит опрос лимитов раньше срока
    pub poke: Mutex<Sender<()>>,
}

/// Ставит окно по пресету. Сверху остров растёт вниз, снизу — вверх,
/// поэтому для нижних пресетов y пересчитывается от высоты.
fn place(win: &WebviewWindow, pos: &str, w: f64, h: f64, free: (f64, f64)) {
    if let Some((x, y)) = place_xy(win, pos, w, h, free) {
        let _ = win.set_position(LogicalPosition::new(x, y));
    }
}

/// Левый верхний угол окна (логические пиксели) для пресета положения.
fn place_xy(win: &WebviewWindow, pos: &str, w: f64, h: f64, free: (f64, f64)) -> Option<(f64, f64)> {
    if pos == "free" {
        return Some(free);
    }
    let Ok(Some(mon)) = win.current_monitor() else {
        return None;
    };
    let sf = mon.scale_factor();
    let ms = mon.size().to_logical::<f64>(sf);
    let mp = mon.position().to_logical::<f64>(sf);
    // «вырез» — всегда по центру и вплотную к верхнему краю экрана
    if pos == "notch" {
        return Some((mp.x + (ms.width - w) / 2.0, mp.y));
    }
    let x = match pos.chars().nth(1) {
        Some('l') => mp.x + 16.0,
        Some('r') => mp.x + ms.width - w - 16.0,
        _ => mp.x + (ms.width - w) / 2.0,
    };
    let y = if pos.starts_with('t') {
        mp.y + 12.0
    } else {
        mp.y + ms.height - h - 56.0
    };
    Some((x, y))
}

/// Скругляет само окно регионом. Размер передаётся явно: set_size применяется
/// асинхронно, и inner_size() в этот момент ещё отдаёт старую геометрию.
#[cfg(windows)]
#[allow(dead_code)]
fn round_corners(win: &WebviewWindow, radius: f64, w: f64, h: f64) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
    let Ok(hh) = win.hwnd() else {
        return;
    };
    let sf = win.scale_factor().unwrap_or(1.0);
    let (pw, ph) = ((w * sf).round() as i32, (h * sf).round() as i32);
    if pw <= 0 || ph <= 0 {
        return;
    }
    unsafe {
        let r = ((radius * sf).round() as i32).max(1);
        let rgn = CreateRoundRectRgn(0, 0, pw + 1, ph + 1, r * 2, r * 2);
        // после успешного вызова регионом владеет система — течь неоткуда
        SetWindowRgn(HWND(hh.0 as _), rgn, true);
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn round_corners(_win: &WebviewWindow, _radius: f64, _w: f64, _h: f64) {}

/// Убирает окно из Alt+Tab и с панели задач.
#[cfg(windows)]
fn tool_window(win: &WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    };
    if let Ok(h) = win.hwnd() {
        unsafe {
            let hwnd = HWND(h.0 as _);
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW.0 as isize);
            // Windows 11 скругляет окно (≈8 px) и рисует вокруг него рамку. У капсулы радиус
            // больше, поэтому рамка торчала дугами в четырёх углах. Край рисует сама капсула.
            use windows::Win32::Graphics::Dwm::{
                DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
            };
            let corner = DWMWCP_DONOTROUND;
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &corner as *const _ as _, 4);
            let none: u32 = 0xFFFF_FFFE; // DWMWA_COLOR_NONE
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &none as *const _ as _, 4);
        }
    }
}

#[cfg(not(windows))]
fn tool_window(_win: &WebviewWindow) {}

/// «Claude думает» из двух источников:
/// — хуки Claude Code кладут файл-метку на сессию, пока идёт ответ (метки старше 15 мин — забытые);
/// — в чатах приложения Claude видна кнопка «Stop» (раз в ~0,8 с через UI Automation).
fn fresh_marker(dir: &std::path::Path) -> bool {
    let now = std::time::SystemTime::now();
    std::fs::read_dir(dir).into_iter().flatten().flatten().any(|e| {
        e.metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age.as_secs() < 900)
    })
}

fn watch_thinking(app: AppHandle, dir: PathBuf) {
    std::thread::spawn(move || {
        let wdir = dir.with_file_name("waiting");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::create_dir_all(&wdir);
        let (mut last, mut last_wait) = (false, false);
        let mut since = std::time::Instant::now();
        #[cfg(windows)]
        let chat = if diag_off("chat") { None } else { chatwatch::Watch::new() };
        let (mut tick, mut chat_busy) = (0u32, false);
        loop {
            tick += 1;
            #[cfg(windows)]
            // чаты Claude проверяем раз в 1,2 с — это межпроцессный запрос к приложению Claude
            if tick % 3 == 0 {
                chat_busy = chat.as_ref().is_some_and(|c| c.busy());
            }
            let wait = fresh_marker(&wdir);
            let on = fresh_marker(&dir) || chat_busy;
            if on != last {
                // «Готово» — только если Claude работал заметное время
                if !on && since.elapsed().as_secs() >= 4 {
                    let _ = app.emit("claude-done", ());
                }
                since = std::time::Instant::now();
            }
            // повтор раз в ~5 с: первое событие могло уйти до загрузки интерфейса
            if on != last || tick % 12 == 0 {
                let _ = app.emit("thinking", on);
            }
            if wait != last_wait || tick % 12 == 0 {
                let _ = app.emit("waiting", wait);
            }
            last = on;
            last_wait = wait;
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
    });
}

/* ---------------- команды ---------------- */

/// Окно покадрово повторяет реальный размер капсулы: пружину рисует CSS,
/// окно лишь следует за её геометрией, поэтому рассинхрона быть не может.
#[tauri::command]
fn set_island_size(app: AppHandle, w: f64, h: f64) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let (pos, free) = {
        let st = app.state::<AppState>();
        let s = st.settings.lock().unwrap();
        (s.place_pos().to_string(), (s.free_x, s.free_y))
    };
    // Размер и положение — одним SetWindowPos. Раньше это были два отдельных вызова на каждом
    // кадре анимации: окно на миг получало новую ширину на старом месте (рывок вбок),
    // а WebView2 перестраивался дважды.
    #[cfg(windows)]
    if let (Some((x, y)), Ok(h_)) = (place_xy(&win, &pos, w, h, free), win.hwnd()) {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER,
        };
        let sf = win.scale_factor().unwrap_or(1.0);
        let r = |v: f64| (v * sf).round() as i32;
        unsafe {
            let _ = SetWindowPos(HWND(h_.0 as _), HWND::default(), r(x), r(y), r(w), r(h),
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
        }
        return;
    }
    let _ = win.set_size(LogicalSize::new(w, h));
    place(&win, &pos, w, h, free);
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(app: AppHandle, state: tauri::State<AppState>, s: Settings) -> Result<(), String> {
    let (old_hk, old_auto) = {
        let mut cur = state.settings.lock().unwrap();
        let old = std::mem::replace(&mut *cur, s.clone());
        (old.hotkey, old.autostart)
    };
    let hk = if old_hk != s.hotkey { apply_hotkey(&app, &s.hotkey) } else { Ok(()) };
    let _ = settings::save(&state.config_dir, &s);
    // раньше вызывалось при каждом сохранении; теперь — только когда галочку переключили
    if old_auto != s.autostart { set_autostart(s.autostart); }
    let _ = state.poke.lock().unwrap().send(());
    if let Some(win) = app.get_webview_window("main") {
        let sf = win.scale_factor().unwrap_or(1.0);
        let size = win
            .inner_size()
            .map(|x| x.to_logical::<f64>(sf))
            .unwrap_or(LogicalSize::new(250.0, 38.0));
        place(&win, s.place_pos(), size.width, size.height, (s.free_x, s.free_y));
    }
    let _ = app.emit("settings-update", &s);
    hk
}

/// Перерегистрирует горячую клавишу. Ошибка — если сочетание занято другой программой.
fn apply_hotkey(app: &AppHandle, hk: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if hk.is_empty() { return Ok(()); }
    gs.register(hk).map_err(|e| {
        ilog!("горячая клавиша {hk}: {e}");
        "busy".to_string()
    })
}

/// Команда обязана быть async: синхронная команда на Windows создаёт окно
/// в главном потоке и намертво его блокирует (белое окно + зависший остров).
#[tauri::command]
async fn open_settings(app: AppHandle) {
    show_settings(&app);
}

fn show_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Claude Island — настройки")
        .inner_size(780.0, 600.0)
        .resizable(false)
        .maximizable(false)
        .center()
        .focused(true)
        .build()
        // поверх остальных окон, иначе при первом запуске оно прячется за браузером
        .map(|w| {
            let _ = w.set_always_on_top(true);
            let _ = w.set_focus();
            let _ = w.set_always_on_top(false);
        });
}

#[tauri::command]
fn has_session_key(state: tauri::State<AppState>) -> bool {
    secret::load(&state.config_dir).is_some()
}

/// Сохраняет ключ (пустая строка — удалить), сразу проверяет и будит опрос.
#[tauri::command]
async fn set_session_key(app: AppHandle, key: String) -> Result<usage::Usage, String> {
    let dir = app.state::<AppState>().config_dir.clone();
    secret::save(&dir, &key)?;
    let _ = app.state::<AppState>().poke.lock().unwrap().send(());
    if key.trim().is_empty() {
        return Ok(usage::Usage::default());
    }
    let k = key.trim().to_string();
    Ok(tauri::async_runtime::spawn_blocking(move || usage::fetch(&k, &mut None))
        .await
        .unwrap_or_default())
}

/// Вход в claude.ai прямо из программы: открываем сайт в отдельном окне, человек входит
/// как обычно, а мы раз в секунду смотрим, появилась ли cookie sessionKey. Нашли —
/// сохраняем ключ (зашифрованным, как и раньше) и закрываем окно. Сайт в этом окне
/// не имеет доступа к командам программы: в capabilities его нет.
#[tauri::command]
async fn claude_login(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("login") {
        let _ = w.set_focus();
        return Ok(());
    }
    let url: tauri::Url = "https://claude.ai/login".parse().map_err(|e| format!("{e}"))?;
    let win = WebviewWindowBuilder::new(&app, "login", WebviewUrl::External(url))
        .title("Claude Island — вход в claude.ai")
        .inner_size(480.0, 760.0)
        .center()
        .focused(true)
        // «Продолжить через Google/Apple» открывают вход во всплывающем окне (window.open)
        // и ждут ответа через window.opener — без этого claude.ai пишет «ошибка при входе»
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Allow)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = win.set_always_on_top(true);
    let _ = win.set_always_on_top(false);
    std::thread::spawn(move || {
        let site: tauri::Url = "https://claude.ai".parse().unwrap();
        // ждём до 15 минут, пока человек входит
        for _ in 0..900 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let Some(w) = app.get_webview_window("login") else { return }; // окно закрыли
            let Ok(cookies) = w.cookies_for_url(site.clone()) else { continue };
            let Some(key) = cookies.iter().find(|c| c.name() == "sessionKey").map(|c| c.value().to_string()) else { continue };
            if !key.starts_with("sk-ant-") { continue; }
            let dir = app.state::<AppState>().config_dir.clone();
            if let Err(e) = secret::save(&dir, &key) { ilog!("вход: не удалось сохранить ключ: {e}"); return; }
            ilog!("вход в claude.ai: ключ получен");
            let _ = app.state::<AppState>().poke.lock().unwrap().send(());
            let u = usage::fetch(&key, &mut None);
            let _ = app.emit("login-done", &u);
            let _ = w.close();
            return;
        }
    });
    Ok(())
}

#[tauri::command]
fn cursor_inside(win: WebviewWindow) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::{POINT, RECT};
        use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
        let (Ok(h), mut p, mut r) = (win.hwnd(), POINT::default(), RECT::default()) else { return true };
        if GetCursorPos(&mut p).is_err() || GetWindowRect(windows::Win32::Foundation::HWND(h.0 as _), &mut r).is_err() { return true; }
        return p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom;
    }
    #[allow(unreachable_code)]
    true
}

#[tauri::command]
fn refresh_usage(state: tauri::State<AppState>) {
    let _ = state.poke.lock().unwrap().send(());
}

/// Автозапуск через HKCU\...\Run — без прав администратора.
#[cfg(windows)]
fn set_autostart(on: bool) {
    use windows::core::w;
    use windows::Win32::System::Registry::{RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
    let run = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
    unsafe {
        if on {
            let Ok(exe) = std::env::current_exe() else { return };
            let v: Vec<u16> = format!("\"{}\" --autostart", exe.display()).encode_utf16().chain([0]).collect();
            let _ = RegSetKeyValueW(HKEY_CURRENT_USER, run, w!("ClaudeIsland"), REG_SZ.0,
                Some(v.as_ptr() as _), (v.len() * 2) as u32);
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, run, w!("ClaudeIsland"));
        }
    }
    guard::set_logon_task(on);
}

#[cfg(not(windows))]
fn set_autostart(_on: bool) {}

#[tauri::command]
fn media_control(action: String) {
    #[cfg(windows)]
    media::control(action);
    #[cfg(not(windows))]
    let _ = action;
}

#[tauri::command]
fn get_media_cover() -> serde_json::Value {
    #[cfg(windows)]
    { serde_json::to_value(media::current_cover()).unwrap_or_default() }
    #[cfg(not(windows))]
    serde_json::Value::Null
}

#[tauri::command]
fn tg_diag() -> serde_json::Value {
    #[cfg(windows)]
    { serde_json::to_value(tg::diag()).unwrap_or_default() }
    #[cfg(not(windows))]
    serde_json::Value::Null
}

/// Открывает раздел Параметров Windows (только из белого списка).
#[tauri::command]
fn open_win_settings(page: String) {
    let uri = match page.as_str() {
        "notifications" => "ms-settings:notifications",
        "privacy-notifications" => "ms-settings:privacy-notifications",
        _ => return,
    };
    let _ = std::process::Command::new("explorer.exe").arg(uri).spawn();
}

#[tauri::command]
fn mark_tg_read() {
    #[cfg(windows)]
    tg::clear();
}

/* ---------------- запуск ---------------- */

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// До 0.2.1 папка настроек называлась com.oligarh.claudeisland — переносим
/// настройки, ключ сессии и журнал в новую, чтобы обновление ничего не потеряло.
fn migrate_config(dir: &std::path::Path) {
    // ветка реестра, которую оставил установщик 0.2.0
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, windows::core::w!(r"Software\oligarh"));
    }
    let old = dir.with_file_name("com.oligarh.claudeisland");
    if !old.is_dir() || dir.join("settings.json").exists() { return; }
    let _ = std::fs::create_dir_all(dir);
    for f in ["settings.json", "session.key", "island.log"] {
        let _ = std::fs::copy(old.join(f), dir.join(f));
    }
    // копия удалась — старую папку убираем целиком
    if dir.join("settings.json").exists() { let _ = std::fs::remove_dir_all(&old); }
    // и кэш WebView старой версии
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let _ = std::fs::remove_dir_all(std::path::Path::new(&local).join("com.oligarh.claudeisland"));
    }
}

/// Диагностика: CI_OFF=chat,tg,media,sys,volume,usage выключает подсистемы,
/// чтобы по очереди измерить, какая из них нагружает процессор.
fn diag_off(name: &str) -> bool {
    std::env::var("CI_OFF").is_ok_and(|v| v.split(',').any(|x| x.trim() == name || x.trim() == "all"))
}

pub fn watch(pid: u32, restarts: u32) {
    #[cfg(windows)]
    guard::watch(pid, restarts);
    #[cfg(not(windows))]
    let _ = (pid, restarts);
}

pub fn hook(mode: &str) {
    hooks::run(mode);
}

#[derive(serde::Serialize)]
struct HookState { on: bool, current: bool }

#[tauri::command]
fn hooks_status() -> HookState {
    let (on, current) = hooks::status();
    HookState { on, current }
}

#[tauri::command]
fn set_hooks(on: bool) -> Result<(), String> {
    let r = hooks::install(on);
    if let Err(e) = &r { ilog!("хуки: {e}"); }
    r
}

#[tauri::command]
fn open_log(app: AppHandle) {
    use tauri_plugin_opener::OpenerExt;
    if let Some(p) = log::path() {
        let _ = app.opener().reveal_item_in_dir(p);
    }
}

/// Проверка обновлений: при запуске и раз в 6 часов. Найдено — сообщаем острову и настройкам.
fn watch_updates(app: AppHandle) {
    use tauri_plugin_updater::UpdaterExt;
    tauri::async_runtime::spawn(async move {
        loop {
            let on = app.state::<AppState>().settings.lock().unwrap().auto_update;
            if on {
                match app.updater() {
                    Ok(u) => match u.check().await {
                        Ok(Some(up)) => { let _ = app.emit("update-available", up.version.clone()); }
                        Ok(None) => {}
                        Err(e) => ilog!("обновления: {e}"),
                    },
                    Err(e) => ilog!("обновления: {e}"),
                }
            }
            tokio_sleep(6 * 3600).await;
        }
    });
}

async fn tokio_sleep(secs: u64) {
    let (tx, rx) = tauri::async_runtime::channel::<()>(1);
    std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_secs(secs)); let _ = tx.blocking_send(()); });
    let mut rx = rx;
    let _ = rx.recv().await;
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let up = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| { ilog!("обновления: {e}"); e.to_string() })?;
    Ok(up.map(|u| u.version))
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let Some(up) = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())? else {
        return Err("none".into());
    };
    ilog!("ставлю обновление {}", up.version);
    up.download_and_install(|_, _| {}, || {}).await.map_err(|e| { ilog!("обновление не встало: {e}"); e.to_string() })?;
    app.restart();
}

#[tauri::command]
fn app_version() -> &'static str { env!("CARGO_PKG_VERSION") }

pub fn run() {
    tauri::Builder::default()
        // второй запуск (ярлык + автозапуск) не плодит второй остров, а открывает настройки
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !args.iter().any(|a| a == "--autostart") { show_settings(app); }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _sc, ev| {
                    if ev.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = app.emit("toggle-hide", ());
                    }
                })
                .build(),
        )
        .setup(|app| {
            let config_dir = app.path().app_config_dir().unwrap_or_default();
            migrate_config(&config_dir);
            log::init(&config_dir);
            #[cfg(windows)]
            {
                guard::mark_alive();
                if !cfg!(debug_assertions) {
                    let args: Vec<String> = std::env::args().collect();
                    let n = args.iter().position(|a| a == "--restarts")
                        .and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0);
                    guard::spawn_watchdog(n);
                }
            }
            hooks::refresh();
            let s = settings::load(&config_dir);
            // автозапуск включён — обновляем запись (путь, --autostart) и задачу планировщика:
            // у старых установок её нет, а без неё остров стартует через минуту после входа
            #[cfg(windows)]
            if s.autostart && !cfg!(debug_assertions) {
                let has_task = std::process::Command::new("schtasks")
                    .args(["/query", "/tn", "Claude Island"])
                    .creation_flags(0x0800_0000)
                    .status().is_ok_and(|st| st.success());
                if !has_task { set_autostart(true); }
            }
            let (tx, rx) = channel();
            app.manage(AppState {
                settings: Mutex::new(s.clone()),
                config_dir,
                hidden: AtomicBool::new(false),
                poke: Mutex::new(tx),
            });
            if !diag_off("usage") { usage::start(app.handle().clone(), rx); }
            // первый запуск без ключа — сразу показываем настройки с инструкцией
            if secret::load(&app.state::<AppState>().config_dir).is_none() {
                show_settings(app.handle());
            }
            watch_thinking(app.handle().clone(), hooks::base().join("thinking"));

            if let Some(win) = app.get_webview_window("main") {
                tool_window(&win);
                // заголовок прозрачного окна Windows порой рисует поверх капсулы — убираем его
                let _ = win.set_title("");
                // Системное размытие не годится: Acrylic/Mica у неактивного окна становятся
                // сплошной заливкой, а blur-behind игнорирует скруглённый регион и даёт
                // чёрные углы. Матовое стекло рисует CSS.
                place(&win, s.place_pos(), 250.0, 38.0, (s.free_x, s.free_y));
                let _ = win.emit("settings-update", &s);
            }

            #[cfg(windows)]
            if !diag_off("media") { media::start(app.handle().clone()); }
            #[cfg(windows)]
            if !diag_off("volume") { volume::start(app.handle().clone()); }
            #[cfg(windows)]
            if !diag_off("sys") { sys::start(app.handle().clone()); }
            watch_updates(app.handle().clone());
            // горячая клавиша «показать/спрятать» (по умолчанию Win+Alt+C)
            let hk = app.state::<AppState>().settings.lock().unwrap().hotkey.clone();
            let _ = apply_hotkey(app.handle(), &hk);
            #[cfg(windows)]
            if !diag_off("tg") { tg::start(app.handle().clone()); }

            let lang = app.state::<AppState>().settings.lock().unwrap().lang.clone();
            #[cfg(windows)]
            let sys_ru = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() } & 0x3ff == 0x19;
            #[cfg(not(windows))]
            let sys_ru = true;
            let en = lang == "en" || (lang != "ru" && !sys_ru);
            let tr = |ru: &'static str, e: &'static str| if en { e } else { ru };
            let i_settings = MenuItem::with_id(app, "settings", tr("Настройки", "Settings"), true, None::<&str>)?;
            let i_toggle = MenuItem::with_id(app, "toggle", tr("Скрыть / показать", "Hide / show"), true, None::<&str>)?;
            let i_quit = MenuItem::with_id(app, "quit", tr("Выход", "Quit"), true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&i_settings, &i_toggle, &i_quit])?;

            let mut tray = TrayIconBuilder::new().tooltip("Claude Island").menu(&menu);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, ev| match ev.id.as_ref() {
                "quit" => {
                    ilog!("выход: пункт «Выход» в трее");
                    app.exit(0)
                }
                "toggle" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.emit("toggle-hidden", ());
                    }
                }
                "settings" => show_settings(app),
                _ => {}
            })
            .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cursor_inside,
            tg_diag,
            open_win_settings,
            claude_login,
            get_media_cover,
            check_update,
            install_update,
            app_version,
            hooks_status,
            set_hooks,
            open_log,
            set_island_size,
            get_settings,
            save_settings,
            open_settings,
            media_control,
            mark_tg_read,
            has_session_key,
            set_session_key,
            refresh_usage
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, ev| match ev {
            // остров нельзя закрыть случайно (Alt+F4 и т.п.): без окон приложение бы завершилось
            tauri::RunEvent::WindowEvent { label, event: tauri::WindowEvent::CloseRequested { api, .. }, .. } if label == "main" => {
                api.prevent_close();
                ilog!("попытка закрыть остров — оставляем его работать");
                if let Some(w) = app.get_webview_window("main") { let _ = w.emit("toggle-hidden", ()); }
            }
            tauri::RunEvent::ExitRequested { code, .. } => ilog!("запрошен выход, код {code:?}"),
            tauri::RunEvent::Exit => ilog!("выход"),
            _ => {}
        });
}
