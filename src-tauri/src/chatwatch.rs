//! «Claude думает» в чатах приложения Claude: пока идёт ответ, в окне есть кнопка
//! «Stop». Читаем её через UI Automation (так же, как экранные дикторы) — ~20 мс.

use windows::core::{w, VARIANT};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationCondition, TreeScope_Descendants,
    UIA_ButtonControlTypeId, UIA_ControlTypePropertyId,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, IsWindowVisible};

/// Подписи кнопки остановки в разных версиях и языках интерфейса.
const STOP: &[&str] = &["Stop", "Stop response", "Stop generating", "Остановить", "Остановить ответ"];

pub struct Watch {
    ua: IUIAutomation,
    buttons: IUIAutomationCondition,
}

impl Watch {
    /// Создаётся в том потоке, где будет опрашиваться.
    pub fn new() -> Option<Self> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let ua: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let buttons = ua
                .CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_ButtonControlTypeId.0))
                .ok()?;
            Some(Self { ua, buttons })
        }
    }

    /// Окон «Claude» несколько (есть скрытое служебное) — проверяем все видимые.
    pub fn busy(&self) -> bool {
        let mut after = HWND::default();
        unsafe {
            while let Ok(h) = FindWindowExW(HWND::default(), after, w!("Chrome_WidgetWin_1"), w!("Claude")) {
                if h.is_invalid() {
                    break;
                }
                after = h;
                if IsWindowVisible(h).as_bool() && self.window_busy(h) {
                    return true;
                }
            }
        }
        false
    }

    fn window_busy(&self, hwnd: HWND) -> bool {
        unsafe {
            let Ok(root) = self.ua.ElementFromHandle(hwnd) else { return false };
            let Ok(all) = root.FindAll(TreeScope_Descendants, &self.buttons) else { return false };
            for i in 0..all.Length().unwrap_or(0) {
                if let Ok(name) = all.GetElement(i).and_then(|e| e.CurrentName()) {
                    if STOP.contains(&name.to_string().as_str()) {
                        return true;
                    }
                }
            }
            false
        }
    }
}
