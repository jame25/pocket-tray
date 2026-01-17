//! Clipboard monitoring thread

use crate::tts::TTSCommand;
use arboard::Clipboard;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

/// Clipboard monitor configuration
const POLL_INTERVAL_MS: u64 = 500;

/// Clipboard monitor running in a dedicated thread
pub struct ClipboardMonitor {
    enabled: Arc<AtomicBool>,
    is_speaking: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    tts_tx: Sender<TTSCommand>,
    last_text: String,
}

impl ClipboardMonitor {
    /// Create a new clipboard monitor
    pub fn new(
        enabled: Arc<AtomicBool>,
        is_speaking: Arc<AtomicBool>,
        shutdown: Arc<AtomicBool>,
        tts_tx: Sender<TTSCommand>,
    ) -> Self {
        Self {
            enabled,
            is_speaking,
            shutdown,
            tts_tx,
            last_text: String::new(),
        }
    }

    /// Run the monitoring loop
    pub fn run(&mut self) {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to access clipboard: {}", e);
                return;
            }
        };

        // Initialize last_text with current clipboard content to avoid speaking it at launch
        if let Ok(text) = clipboard.get_text() {
            self.last_text = text.trim().to_string();
            log::info!("Initialized with existing clipboard content ({} chars)", self.last_text.len());
        }

        log::info!("Clipboard monitor started");

        loop {
            // Check for shutdown
            if self.shutdown.load(Ordering::Relaxed) {
                log::info!("Clipboard monitor shutting down");
                break;
            }

            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

            // Check if monitoring is enabled
            if !self.enabled.load(Ordering::Relaxed) {
                continue;
            }

            // Check if currently speaking (ignore new text per user requirement)
            if self.is_speaking.load(Ordering::Relaxed) {
                continue;
            }

            // Get clipboard text
            let text = match clipboard.get_text() {
                Ok(t) => t,
                Err(_) => continue, // Not text content or clipboard error
            };

            // Check if it's new text and not empty
            let text = text.trim().to_string();
            if text == self.last_text || text.is_empty() {
                continue;
            }

            // Check text is reasonable length (avoid giant pastes)
            if text.len() > 10000 {
                log::warn!("Clipboard text too long ({} chars), ignoring", text.len());
                self.last_text = text;
                continue;
            }

            // Store and speak
            log::info!("New clipboard text detected ({} chars)", text.len());
            self.last_text = text.clone();

            // Send to TTS thread
            if let Err(e) = self.tts_tx.send(TTSCommand::Speak { text }) {
                log::error!("Failed to send TTS command: {}", e);
                break; // Channel closed
            }
        }
    }
}

/// Spawn the clipboard monitor in a separate thread
pub fn spawn_clipboard_thread(
    enabled: Arc<AtomicBool>,
    is_speaking: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    tts_tx: Sender<TTSCommand>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("clipboard-monitor".into())
        .spawn(move || {
            let mut monitor = ClipboardMonitor::new(enabled, is_speaking, shutdown, tts_tx);
            monitor.run();
        })
        .expect("Failed to spawn clipboard thread")
}
