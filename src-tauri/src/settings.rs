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
    /// pill (капсула) | notch (вырез сверху экрана)
    pub shape: String,
    /// auto | ru | en
    pub lang: String,
    /// сочетание в формате tauri (super+alt+KeyC); пусто — выключено
    pub hotkey: String,
    /// отдельные эффекты острова
    pub fx_volume: bool,
    pub fx_copy: bool,
    pub fx_power: bool,
    pub fx_audio: bool,
    pub fx_done: bool,
    pub fx_wait: bool,
    pub fx_layout: bool,
    pub fx_privacy: bool,
    pub fx_media: bool,
    /// прятать остров, пока открыто полноэкранное приложение
    pub hide_fullscreen: bool,
    /// длительность помодоро, мин
    pub pomodoro_min: u32,
    pub auto_update: bool,
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
            shape: "pill".into(),
            lang: "auto".into(),
            hotkey: "super+alt+KeyC".into(),
            fx_volume: true,
            fx_copy: true,
            fx_power: true,
            fx_audio: true,
            fx_done: true,
            fx_wait: true,
            fx_layout: true,
            fx_privacy: true,
            fx_media: true,
            hide_fullscreen: true,
            pomodoro_min: 25,
            auto_update: true,
        }
    }
}

impl Settings {
    /// Положение для расчёта окна: вырез всегда сверху по центру.
    pub fn place_pos(&self) -> &str {
        if self.shape == "notch" { "notch" } else { &self.pos }
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
