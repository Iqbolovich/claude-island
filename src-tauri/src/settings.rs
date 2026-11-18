use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    /// tl tc tr bl bc br | free
    pub pos: String,
    pub free_x: f64,
    pub free_y: f64,
    /// пороги предупреждений, %
    pub warn_at: u8,
    pub alarm_at: u8,
    /// прозрачность капсулы, 0..1
    pub opacity: f64,
    pub show_music: bool,
    pub show_telegram: bool,
    /// период чередования «таймер ↔ часы», с
    pub swap_secs: u32,
    /// период опроса лимитов, с
    pub poll_secs: u64,
    pub autostart: bool,
    /// black | glass
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pos: "tc".into(),
            free_x: 0.0,
            free_y: 0.0,
            warn_at: 80,
            alarm_at: 95,
            opacity: 0.55,
            show_music: true,
            show_telegram: true,
            swap_secs: 4,
            poll_secs: 60,
            autostart: false,
            theme: "black".into(),
        }
    }
}

fn file(dir: &PathBuf) -> PathBuf {
    dir.join("settings.json")
}

pub fn load(dir: &PathBuf) -> Settings {
    std::fs::read_to_string(file(dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &PathBuf, s: &Settings) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(file(dir), serde_json::to_string_pretty(s).unwrap_or_default())
}
