//! Pocket-Tray: Windows System Tray TTS Application
//!
//! A standalone Windows application that monitors the clipboard and speaks
//! copied text using the Pocket TTS engine.

#![windows_subsystem = "windows"]

mod app;
mod clipboard;
mod icon;
mod settings;
mod tray;
mod tts;

use app::App;
use settings::Settings;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    log::info!("Pocket-Tray starting...");

    // Load settings
    let settings = Settings::load_or_default();
    log::info!(
        "Settings loaded: monitor={}, voice={}",
        settings.monitor_enabled,
        settings.current_voice
    );

    // Create and run application
    match App::new(settings) {
        Ok(app) => {
            if let Err(e) = app.run() {
                log::error!("Application error: {}", e);
                show_error_message(&format!("Application error: {}", e));
            }
        }
        Err(e) => {
            log::error!("Failed to initialize application: {}", e);
            show_error_message(&format!("Failed to initialize: {}", e));
        }
    }

    log::info!("Pocket-Tray exiting");
}

/// Show an error message dialog on Windows
#[cfg(windows)]
fn show_error_message(message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let title: Vec<u16> = OsStr::new("Pocket-Tray Error")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let text: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        MessageBoxW(
            None,
            PCWSTR::from_raw(text.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_error_message(message: &str) {
    eprintln!("Error: {}", message);
}
