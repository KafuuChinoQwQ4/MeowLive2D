#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    #[cfg(windows)]
    {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            meowlive_desktop::startup::record_startup_error(&info.to_string());
            previous(info);
        }));
    }
    match meowlive_desktop::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            #[cfg(windows)]
            meowlive_desktop::startup::record_startup_error(&error);
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
