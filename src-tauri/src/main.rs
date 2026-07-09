#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if should_route_to_cli(&args) {
        let code = winbox_gui_lib::cli::run();
        std::process::exit(code);
    }
    winbox_gui_lib::run();
}

fn should_route_to_cli(args: &[String]) -> bool {
    args.len() > 1
        && !args
            .get(1)
            .is_some_and(|arg| arg == "--window=office-progress")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_progress_window_argv_routes_to_gui() {
        let args = vec![
            "winbox".to_string(),
            "--window=office-progress".to_string(),
            "office".to_string(),
            "excel".to_string(),
        ];

        assert!(!should_route_to_cli(&args));
        assert!(should_route_to_cli(&[
            "winbox".to_string(),
            "office".to_string(),
            "launch".to_string()
        ]));
    }
}
