// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // хук Claude Code: claude-island.exe --hook on|off|wait — отработать и выйти без окна
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--hook") {
        claude_island_lib::hook(args.get(2).map(String::as_str).unwrap_or("off"));
        return;
    }
    // сторож: claude-island.exe --watch <pid> <падений подряд>
    if args.get(1).map(String::as_str) == Some("--watch") {
        let pid = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
        let n = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(0);
        claude_island_lib::watch(pid, n);
        return;
    }
    claude_island_lib::run()
}
