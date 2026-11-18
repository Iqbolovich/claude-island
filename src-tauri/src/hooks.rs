//! Связь с Claude Code через хуки. Хук запускает этот же exe с ключом --hook,
//! поэтому у других людей не нужны ни Git Bash, ни папка проекта.
//!   on   — UserPromptSubmit / PostToolUse: Claude работает
//!   off  — Stop / SessionEnd: закончил
//!   wait — Notification: если просит разрешения

use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;

const MARK: &str = "--hook";

fn base() -> PathBuf {
    let app = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(app).join("com.oligarh.claudeisland")
}

/// Вызывается из main до запуска окна: делает своё и выходит.
pub fn run(mode: &str) {
    let mut buf = Vec::new();
    let _ = std::io::stdin().take(256 * 1024).read_to_end(&mut buf);
    let v: Value = serde_json::from_slice(&buf).unwrap_or(Value::Null);
    let id: String = v["session_id"].as_str().unwrap_or("default")
        .chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    let id = if id.is_empty() { "default".into() } else { id };
    let (t, w) = (base().join("thinking"), base().join("waiting"));
    let _ = std::fs::create_dir_all(&t);
    let _ = std::fs::create_dir_all(&w);
    match mode {
        "on" => {
            let _ = std::fs::write(t.join(&id), b"");
            let _ = std::fs::remove_file(w.join(&id));
        }
        "wait" => {
            let s = String::from_utf8_lossy(&buf).to_lowercase();
            if s.contains("permission") {
                let _ = std::fs::write(w.join(&id), b"");
            }
        }
        _ => {
            let _ = std::fs::remove_file(t.join(&id));
            let _ = std::fs::remove_file(w.join(&id));
        }
    }
}

fn settings_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    PathBuf::from(home).join(".claude").join("settings.json")
}

fn exe() -> String {
    std::env::current_exe().map(|p| p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/")).unwrap_or_default()
}

fn ours(cmd: &str) -> bool {
    cmd.contains("claude-island") || cmd.contains("thinking.sh")
}

fn read() -> Value {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}

/// Состояние: подключено ли, и указывает ли хук на этот exe.
pub fn status() -> (bool, bool) {
    let txt = std::fs::read_to_string(settings_path()).unwrap_or_default();
    let on = txt.contains("claude-island") && txt.contains(MARK) || txt.contains("thinking.sh");
    let current = txt.replace("\\\\", "/").contains(&exe());
    (on, current)
}

/// Добавить (on) или убрать хуки острова. Чужие хуки не трогаем.
pub fn install(on: bool) -> Result<(), String> {
    let path = settings_path();
    let mut root = read();
    if !root.is_object() { return Err("~/.claude/settings.json повреждён".into()); }
    if path.exists() {
        let bak = path.with_file_name("settings.json.island-backup");
        if !bak.exists() { let _ = std::fs::copy(&path, &bak); }
    }
    let hooks = root.as_object_mut().unwrap().entry("hooks").or_insert_with(|| json!({}));
    let Some(map) = hooks.as_object_mut() else { return Err("поле hooks не объект".into()) };
    // убираем старые записи острова (в том числе прежний thinking.sh)
    for (_, list) in map.iter_mut() {
        if let Some(arr) = list.as_array_mut() {
            arr.retain(|m| !m["hooks"].as_array().is_some_and(|hs| hs.iter().any(|h| h["command"].as_str().is_some_and(ours))));
        }
    }
    map.retain(|_, l| l.as_array().is_none_or(|a| !a.is_empty()));
    if on {
        let exe = exe();
        for (ev, mode) in [("UserPromptSubmit", "on"), ("PostToolUse", "on"), ("Stop", "off"), ("SessionEnd", "off"), ("Notification", "wait")] {
            let entry = json!({ "hooks": [{ "type": "command", "command": format!("\"{exe}\" {MARK} {mode}"), "timeout": 5 }] });
            let list = map.entry(ev).or_insert_with(|| json!([]));
            if let Some(a) = list.as_array_mut() { a.push(entry); }
        }
    }
    if map.is_empty() { root.as_object_mut().unwrap().remove("hooks"); }
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let txt = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    std::fs::write(&path, txt).map_err(|e| e.to_string())
}

/// При запуске: если хуки уже подключены, но указывают на старый путь или на
/// thinking.sh — тихо переписываем на этот exe.
pub fn refresh() {
    if cfg!(debug_assertions) { return; } // отладочная сборка не должна перехватывать хуки
    let (on, current) = status();
    if on && !current {
        match install(true) {
            Ok(()) => crate::ilog!("хуки Claude Code обновлены на {}", exe()),
            Err(e) => crate::ilog!("не удалось обновить хуки: {e}"),
        }
    }
}
