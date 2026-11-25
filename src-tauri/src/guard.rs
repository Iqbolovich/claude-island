//! Сторож и быстрый автозапуск.
//!
//! Сторож — тот же exe с ключом `--watch <pid> <n>`: ждёт, пока остров завершится.
//! Вышел через «Выход» (код 0) — сторож тоже выходит. Упал или был убит — через 4 с
//! запускает остров заново (если другой экземпляр уже не поднялся сам). Больше 5
//! падений подряд (каждое меньше чем через 2 минуты после старта) — сдаётся.

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    CreateMutexW, GetExitCodeProcess, OpenMutexW, OpenProcess, WaitForSingleObject, INFINITE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, SYNCHRONIZATION_SYNCHRONIZE,
};

const NO_WINDOW: u32 = 0x0800_0000;

/// Метка «главный экземпляр жив»: держится всё время работы острова.
pub fn mark_alive() {
    unsafe {
        // хэндл намеренно не закрываем — мьютекс живёт, пока жив процесс
        let _ = CreateMutexW(None, false, w!("Local\\ClaudeIslandMain"));
    }
}

fn main_alive() -> bool {
    unsafe {
        match OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, w!("Local\\ClaudeIslandMain")) {
            Ok(h) => { let _ = CloseHandle(h); true }
            Err(_) => false,
        }
    }
}

/// Запускается главным процессом при старте.
pub fn spawn_watchdog(restarts: u32) {
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = Command::new(exe)
        .args(["--watch", &std::process::id().to_string(), &restarts.to_string()])
        .creation_flags(NO_WINDOW)
        .spawn();
}

/// Режим сторожа (из main до запуска окна).
pub fn watch(pid: u32, restarts: u32) {
    let started = Instant::now();
    let code = unsafe {
        let Ok(h): windows::core::Result<HANDLE> = OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return };
        WaitForSingleObject(h, INFINITE);
        let mut c = 0u32;
        let _ = GetExitCodeProcess(h, &mut c);
        let _ = CloseHandle(h);
        c
    };
    if code == 0 {
        return; // вышли сами, через «Выход»
    }
    // поработал дольше 2 минут — счётчик падений с нуля
    let n = if started.elapsed() > Duration::from_secs(120) { 1 } else { restarts + 1 };
    if n > 5 {
        crate::log::write_to_default(&format!("сторож: остров падает раз за разом ({n}), больше не перезапускаю"));
        return;
    }
    std::thread::sleep(Duration::from_secs(4));
    if main_alive() {
        return; // остров уже подняли (обновление, ручной запуск)
    }
    crate::log::write_to_default(&format!("сторож: остров завершился с кодом {code:#x}, перезапускаю ({n})"));
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe).args(["--autostart", "--restarts", &n.to_string()]).spawn();
    }
}

/// Быстрый автозапуск: задача планировщика «при входе в систему» стартует сразу,
/// а запись в Run Windows запускает примерно через минуту. Run остаётся запасным.
pub fn set_logon_task(on: bool) {
    let script = if on {
        let Ok(exe) = std::env::current_exe() else { return };
        format!(
            "$u=\"$env:USERDOMAIN\\$env:USERNAME\"; \
             $t=New-ScheduledTaskTrigger -AtLogOn -User $u; \
             $a=New-ScheduledTaskAction -Execute '{}' -Argument '--autostart'; \
             $p=New-ScheduledTaskPrincipal -UserId $u -LogonType Interactive -RunLevel Limited; \
             $s=New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit 0; \
             Register-ScheduledTask -TaskName 'Claude Island' -Trigger $t -Action $a -Principal $p -Settings $s -Force | Out-Null",
            exe.display().to_string().replace('\'', "''")
        )
    } else {
        "Unregister-ScheduledTask -TaskName 'Claude Island' -Confirm:$false -ErrorAction SilentlyContinue".to_string()
    };
    std::thread::spawn(move || {
        let ok = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .creation_flags(NO_WINDOW)
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            crate::ilog!("автозапуск: задачу планировщика {} не удалось", if on { "создать" } else { "удалить" });
        }
    });
}
