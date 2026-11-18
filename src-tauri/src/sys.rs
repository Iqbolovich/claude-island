//! Мелкие системные события: зарядка и батарея, буфер обмена.

use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Serialize, Clone)]
struct Power { charging: bool, pct: u32, low: bool }

fn power() -> Option<(bool, u32)> {
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).ok()? };
    // 128 — батареи нет (настольный ПК), 255 — неизвестно
    if s.BatteryFlag == 128 || s.BatteryFlag == 255 || s.BatteryLifePercent > 100 { return None; }
    Some((s.ACLineStatus == 1, s.BatteryLifePercent as u32))
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut clip = unsafe { GetClipboardSequenceNumber() };
        let mut pw = power();
        let mut warned = false;
        let mut tick = 0u32;
        loop {
            let c = unsafe { GetClipboardSequenceNumber() };
            if c != clip {
                clip = c;
                let _ = app.emit("copied", ());
            }
            if tick % 5 == 0 {
                let now = power();
                if let (Some((was, _)), Some((ac, pct))) = (pw, now) {
                    if ac != was {
                        let _ = app.emit("power", Power { charging: ac, pct, low: false });
                    }
                    if !ac && pct <= 15 && !warned {
                        warned = true;
                        let _ = app.emit("power", Power { charging: false, pct, low: true });
                    }
                    if ac || pct > 20 { warned = false; }
                }
                pw = now;
            }
            tick = tick.wrapping_add(1);
            std::thread::sleep(Duration::from_millis(300));
        }
    });
}
