//! Telegram через системные уведомления Windows: читаем то, что Telegram Desktop
//! кладёт в центр уведомлений. Прочитал чат в Telegram — он сам убирает уведомления,
//! и бейдж на острове гаснет.

use serde::Serialize;
use std::collections::HashSet;
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
}

// id уведомлений Telegram, которые сейчас в центре уведомлений — чтобы убрать их по клику
thread_local! { static DBG: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new()); }
static IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn read(l: &Listener) -> windows::core::Result<Vec<(u32, Msg)>> {
    let list = l.GetNotificationsAsync(NotificationKinds::Toast)?.get()?;
    let generic = KnownNotificationBindings::ToastGeneric()?;
    let mut out = Vec::new();
    for n in list {
        let name = n
            .AppInfo()
            .and_then(|a| a.DisplayInfo())
            .and_then(|d| d.DisplayName())
            .map(|s| s.to_string())
            .unwrap_or_default();
        DBG.with(|d| { if d.borrow_mut().insert(name.clone()) { crate::ilog!("[tg] приложение в уведомлениях: {name}"); } });
        if !name.to_lowercase().contains("telegram") {
            continue;
        }
        let Ok(b) = n.Notification().and_then(|x| x.Visual()).and_then(|v| v.GetBinding(&generic)) else {
            continue;
        };
        let texts: Vec<String> = b
            .GetTextElements()
            .map(|t| t.into_iter().filter_map(|x| x.Text().ok()).map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let from = texts.first().cloned().unwrap_or_else(|| "Telegram".into());
        let text = texts.get(1..).map(|t| t.join(" ")).unwrap_or_default();
        out.push((n.Id()?, Msg { from, text }));
    }
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
        let mut tick = 0u32;
        loop {
            tick += 1;
            let on = app.state::<crate::AppState>().settings.lock().unwrap().show_telegram;
            let mut st = State { status: "off".into(), items: vec![] };
            if on && access == Access::Allowed {
                st.status = "ok".into();
                let got = match read(&l) { Ok(g) => g, Err(e) => { crate::ilog!("[tg] ошибка чтения: {e}"); vec![] } };
                *IDS.lock().unwrap() = got.iter().map(|(id, _)| *id).collect();
                for (id, m) in &got {
                    // при запуске старые уведомления не «выстреливают»
                    if seen.insert(*id) && !first {
                        let _ = app.emit("tg-notification", m);
                    }
                }
                st.items = got.into_iter().map(|(_, m)| m).collect();
                first = false;
            } else if on {
                st.status = "denied".into();
                access = l.GetAccessStatus().unwrap_or(access);
            }
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
