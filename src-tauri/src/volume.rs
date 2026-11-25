//! Громкость системы. Windows сама сообщает об изменении громкости (callback), поэтому
//! опрос 8 раз в секунду больше не нужен. Раз в 2 с проверяем только, не сменилось ли
//! устройство вывода (подключили наушники) — тогда переподписываемся на новое.

use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::core::implement;
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};

#[derive(Serialize, Clone, PartialEq)]
struct Vol { level: u32, muted: bool }

#[implement(IAudioEndpointVolumeCallback)]
struct Cb {
    app: AppHandle,
    last: Mutex<Option<Vol>>,
}

impl IAudioEndpointVolumeCallback_Impl for Cb_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        let Some(d) = (unsafe { data.as_ref() }) else { return Ok(()) };
        let v = Vol { level: (d.fMasterVolume * 100.0).round() as u32, muted: d.bMuted.as_bool() };
        let mut last = self.last.lock().unwrap();
        if last.as_ref() != Some(&v) {
            let _ = self.app.emit("volume", v.clone());
            *last = Some(v);
        }
        Ok(())
    }
}

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
        let mut cur: Option<(IAudioEndpointVolume, IAudioEndpointVolumeCallback)> = None;
        let mut dev_name: Option<String> = None;
        loop {
            match endpoint() {
                Ok((ep, name)) if dev_name.as_deref() != Some(name.as_str()) || cur.is_none() => {
                    if dev_name.is_some() && !name.is_empty() && dev_name.as_deref() != Some(name.as_str()) {
                        let _ = app.emit("audio-device", name.clone());
                    }
                    // отписываемся от старого устройства и подписываемся на новое
                    if let Some((old, cb)) = cur.take() {
                        unsafe { let _ = old.UnregisterControlChangeNotify(&cb); }
                    }
                    let cb: IAudioEndpointVolumeCallback = Cb { app: app.clone(), last: Mutex::new(read(&ep)) }.into();
                    if unsafe { ep.RegisterControlChangeNotify(&cb) }.is_ok() {
                        cur = Some((ep, cb));
                    }
                    dev_name = Some(name);
                }
                Ok(_) => {}
                Err(_) => {
                    if let Some((old, cb)) = cur.take() {
                        unsafe { let _ = old.UnregisterControlChangeNotify(&cb); }
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}
