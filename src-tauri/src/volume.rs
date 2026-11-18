//! Громкость системы: следим за основным устройством вывода и сообщаем об изменениях.

use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};

#[derive(Serialize, Clone, PartialEq)]
struct Vol { level: u32, muted: bool }

/// Громкость основного устройства вывода и его имя («Наушники (AirPods)»).
fn endpoint() -> windows::core::Result<(IAudioEndpointVolume, String)> {
    unsafe {
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let dev = en.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let name = dev
            .OpenPropertyStore(STGM_READ)
            .and_then(|p| p.GetValue(&PKEY_Device_FriendlyName))
            .map(|v| v.to_string())
            .unwrap_or_default();
        Ok((dev.Activate(CLSCTX_ALL, None)?, name))
    }
}

fn read(ep: &IAudioEndpointVolume) -> Option<Vol> {
    unsafe {
        let l = ep.GetMasterVolumeLevelScalar().ok()?;
        let m = ep.GetMute().ok()?.as_bool();
        Some(Vol { level: (l * 100.0).round() as u32, muted: m })
    }
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        let (mut ep, mut last, mut tick) = (None, None::<Vol>, 0u32);
        let mut dev_name: Option<String> = None;
        loop {
            // устройство вывода может смениться (наушники) — переподключаемся раз в 2 с
            if ep.is_none() || tick % 16 == 0 {
                if let Ok((e, name)) = endpoint() {
                    if dev_name.as_ref().is_some_and(|n| *n != name) && !name.is_empty() {
                        let _ = app.emit("audio-device", name.clone());
                    }
                    dev_name = Some(name);
                    ep = Some(e);
                } else {
                    ep = None;
                }
            }
            if let Some(v) = ep.as_ref().and_then(read) {
                if last.as_ref().is_some_and(|l| *l != v) { let _ = app.emit("volume", v.clone()); }
                last = Some(v);
            }
            tick = tick.wrapping_add(1);
            std::thread::sleep(Duration::from_millis(120));
        }
    });
}
