// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // хук Claude Code: claude-island.exe --hook on|off|wait — отработать и выйти без окна
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--hook") {
        claude_island_lib::hook(args.get(2).map(String::as_str).unwrap_or("off"));
        return;
    }
    claude_island_lib::run()
}
