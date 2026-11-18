//! Музыка через Windows SMTC — работает со Spotify, Яндекс Музыкой, браузером
//! и вообще со всем, что показывает карточку трека в системе.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
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
    /// обложка как data-URL, цвет из неё считает фронтенд
    pub cover: String,
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
        cover: String::new(),
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

fn read_once(cover_key: &mut String, cover_val: &mut String) -> Media {
    let Ok(mgr) = Manager::RequestAsync().and_then(|op| op.get()) else {
        return Media::default();
    };
    let Ok(session) = mgr.GetCurrentSession() else {
        return Media::default();
    };
    let Ok(mut m) = snapshot(&session) else {
        return Media::default();
    };
    let key = format!("{}|{}", m.title, m.artist);
    if key != *cover_key {
        *cover_key = key;
        *cover_val = read_cover(&session).unwrap_or_default();
    }
    m.cover = cover_val.clone();
    m
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut last = Media::default();
        let (mut key, mut cover) = (String::new(), String::new());
        let mut tick = 0u32;
        loop {
            tick += 1;
            let m = read_once(&mut key, &mut cover);
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
