//! «Claude думает» в чатах приложения Claude: пока идёт ответ, в окне есть кнопка
//! «Stop». Ищем её через UI Automation одним запросом: фильтр «кнопка И имя = Stop…»
//! выполняется на стороне Claude, и обратно приходит максимум один элемент.
//! Раньше остров забирал ВСЕ кнопки окна и спрашивал имя каждой отдельным вызовом —
//! с длинным чатом это сотни межпроцессных вызовов раз в 0,8 с.

use windows::core::{w, Interface, VARIANT};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationCondition, TreeScope_Descendants,
    UIA_ButtonControlTypeId, UIA_ControlTypePropertyId, UIA_NamePropertyId,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, IsIconic, IsWindowVisible};

/// Подписи кнопки остановки в разных версиях и языках интерфейса.
const STOP: &[&str] = &["Stop", "Stop response", "Stop generating", "Остановить", "Остановить ответ"];

pub struct Watch {
    ua: IUIAutomation,
    stop_button: IUIAutomationCondition,
}

impl Watch {
    /// Создаётся в том потоке, где будет опрашиваться.
    pub fn new() -> Option<Self> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let ua: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let button = ua
                .CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_ButtonControlTypeId.0))
                .ok()?;
            let mut names: Option<IUIAutomationCondition> = None;
            for n in STOP {
                let c = ua.CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(*n)).ok()?;
                names = Some(match names { None => c, Some(prev) => ua.CreateOrCondition(&prev, &c).ok()? });
            }
            let stop_button = ua.CreateAndCondition(&button, &names?).ok()?;
            Some(Self { ua, stop_button })
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
                if IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool() && !cloaked(h) && self.window_busy(h) {
                    return true;
                }
            }
        }
        false
    }

    fn window_busy(&self, hwnd: HWND) -> bool {
        unsafe {
            let Ok(root) = self.ua.ElementFromHandle(hwnd) else { return false };
            root.FindFirst(TreeScope_Descendants, &self.stop_button).is_ok_and(|e| !e.as_raw().is_null())
        }
    }
}

/// Окно на другом виртуальном рабочем столе или скрытое системой — не опрашиваем.
fn cloaked(h: HWND) -> bool {
    let mut v: u32 = 0;
    unsafe { DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut v as *mut u32 as _, 4) }.is_ok() && v != 0
}
