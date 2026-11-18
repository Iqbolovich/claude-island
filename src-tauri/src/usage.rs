//! Лимиты Claude: тот же запрос, которым пользуется страница claude.ai → Settings → Usage.

use crate::{secret, AppState};
use serde::Serialize;
use serde_json::Value;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const BASE: &str = "https://claude.ai/api";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Serialize, Clone, Default)]
pub struct Usage {
    pub ok: bool,
    pub pct: f64,
    pub week_pct: f64,
    /// ISO-время сброса, фронтенд разбирает его сам
    pub reset_at: String,
    pub week_reset_at: String,
    pub error: String,
}

fn fail(msg: &str) -> Usage {
    Usage { error: msg.into(), ..Default::default() }
}

fn get(key: &str, url: &str) -> Result<Value, String> {
    let r = ureq::get(url)
        .set("Cookie", &format!("sessionKey={key}"))
        .set("User-Agent", UA)
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(15))
        .call();
    match r {
        Ok(resp) => resp.into_json().map_err(|_| "Непонятный ответ claude.ai".to_string()),
        Err(ureq::Error::Status(401, _)) => Err("Ключ устарел — обнови его в настройках".into()),
        Err(ureq::Error::Status(c, _)) => Err(format!("claude.ai отказал ({c})")),
        Err(_) => Err("Нет связи с claude.ai".into()),
    }
}

fn parse(v: &Value) -> Option<Usage> {
    let five = v.get("five_hour")?;
    let s = |o: Option<&Value>, k: &str| o.and_then(|x| x.get(k)).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let n = |o: Option<&Value>| o.and_then(|x| x.get("utilization")).and_then(|x| x.as_f64()).unwrap_or(0.0);
    let week = v.get("seven_day");
    Some(Usage {
        ok: true,
        pct: n(Some(five)),
        week_pct: n(week),
        reset_at: s(Some(five), "resets_at"),
        week_reset_at: s(week, "resets_at"),
        error: String::new(),
    })
}

/// Организаций может быть несколько — берём первую, у которой есть лимиты подписки.
pub fn fetch(key: &str, org: &mut Option<String>) -> Usage {
    if let Some(id) = org.clone() {
        match get(key, &format!("{BASE}/organizations/{id}/usage")) {
            Ok(v) => {
                if let Some(u) = parse(&v) {
                    return u;
                }
            }
            Err(e) => return fail(&e),
        }
        *org = None;
    }
    let orgs = match get(key, &format!("{BASE}/organizations")) {
        Ok(v) => v,
        Err(e) => return fail(&e),
    };
    for o in orgs.as_array().into_iter().flatten() {
        let Some(id) = o.get("uuid").and_then(|x| x.as_str()) else { continue };
        if let Ok(v) = get(key, &format!("{BASE}/organizations/{id}/usage")) {
            if let Some(u) = parse(&v) {
                *org = Some(id.to_string());
                return u;
            }
        }
    }
    fail("В аккаунте не нашлось лимитов подписки")
}

/// Опрос в фоне. Сигнал в канал (новый ключ, «обновить сейчас») будит его раньше срока.
pub fn start(app: AppHandle, poke: Receiver<()>) {
    std::thread::spawn(move || {
        let mut org = None;
        loop {
            let st = app.state::<AppState>();
            let secs = st.settings.lock().unwrap().poll_secs.clamp(10, 900);
            let u = match secret::load(&st.config_dir) {
                Some(key) => fetch(&key, &mut org),
                None => fail("Вставь ключ сессии в настройках"),
            };
            let _ = app.emit("usage-update", &u);
            let _ = poke.recv_timeout(Duration::from_secs(secs));
            while poke.try_recv().is_ok() {}
        }
    });
}
