//! Журнал ошибок: island.log в папке настроек. Нужен, чтобы разобраться,
//! если у кого-то что-то не работает. Больше 512 КБ — начинаем заново.

use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

static PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(dir: &PathBuf) {
    let p = dir.join("island.log");
    if std::fs::metadata(&p).is_ok_and(|m| m.len() > 512 * 1024) {
        let _ = std::fs::rename(&p, dir.join("island.old.log"));
    }
    let _ = PATH.set(p);
    std::panic::set_hook(Box::new(|info| write(&format!("PANIC: {info}"))));
    write(&format!("— запуск v{}", env!("CARGO_PKG_VERSION")));
}

pub fn path() -> Option<PathBuf> { PATH.get().cloned() }

pub fn write(msg: &str) {
    let Some(p) = PATH.get() else { return };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // время в UTC: часы:минуты:секунды и номер суток — достаточно для разбора
    let (d, s) = (ts / 86400, ts % 86400);
    let line = format!("[{d} {:02}:{:02}:{:02}Z] {msg}\n", s / 3600, s / 60 % 60, s % 60);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = f.write_all(line.as_bytes());
    }
}

#[macro_export]
macro_rules! ilog { ($($t:tt)*) => { $crate::log::write(&format!($($t)*)) } }
