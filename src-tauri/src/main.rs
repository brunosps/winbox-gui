#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let code = winbox_gui_lib::cli::run();
        std::process::exit(code);
    }
    winbox_gui_lib::run();
}
