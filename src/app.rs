//! Main application coordinator

use crate::clipboard::spawn_clipboard_thread;
use crate::settings::Settings;
use crate::tray::{process_menu_event, MenuAction, TrayManager};
use crate::tts::{spawn_tts_thread, TTSCommand, TTSEvent};
use anyhow::Result;
use muda::MenuEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

/// Animation frame interval (120ms = ~8.3 FPS)
const ANIMATION_INTERVAL: Duration = Duration::from_millis(120);

/// Main application state
pub struct App {
    settings: Settings,
    tray: Option<TrayManager>,
    tts_tx: mpsc::Sender<TTSCommand>,
    tts_event_rx: mpsc::Receiver<TTSEvent>,
    monitor_enabled: Arc<AtomicBool>,
    #[allow(dead_code)]
    is_speaking: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    model_loaded: bool,
    last_animation_tick: Instant,
    _tts_thread: std::thread::JoinHandle<()>,
    _clipboard_thread: std::thread::JoinHandle<()>,
}

impl App {
    /// Create a new application instance
    pub fn new(settings: Settings) -> Result<Self> {
        // Shared state
        let monitor_enabled = Arc::new(AtomicBool::new(settings.monitor_enabled));
        let is_speaking = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        // Channels
        let (tts_tx, tts_rx) = mpsc::channel::<TTSCommand>();
        let (tts_event_tx, tts_event_rx) = mpsc::channel::<TTSEvent>();

        // Spawn TTS thread
        let tts_thread = spawn_tts_thread(
            settings.current_voice.clone(),
            Arc::clone(&is_speaking),
            tts_rx,
            tts_event_tx,
        );

        // Spawn clipboard monitor thread
        let clipboard_thread = spawn_clipboard_thread(
            Arc::clone(&monitor_enabled),
            Arc::clone(&is_speaking),
            Arc::clone(&shutdown),
            tts_tx.clone(),
        );

        Ok(Self {
            settings,
            tray: None,
            tts_tx,
            tts_event_rx,
            monitor_enabled,
            is_speaking,
            shutdown,
            model_loaded: false,
            last_animation_tick: Instant::now(),
            _tts_thread: tts_thread,
            _clipboard_thread: clipboard_thread,
        })
    }

    /// Run the application event loop
    pub fn run(mut self) -> Result<()> {
        let event_loop = EventLoop::new()?;
        // Use Poll mode for animation ticking
        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.run_app(&mut self)?;

        Ok(())
    }

    /// Handle menu events
    fn handle_menu_event(&mut self, event: &MenuEvent) {
        match process_menu_event(event) {
            MenuAction::ToggleMonitor => {
                let new_state = !self.monitor_enabled.load(Ordering::SeqCst);
                self.monitor_enabled.store(new_state, Ordering::SeqCst);
                self.settings.monitor_enabled = new_state;
                if let Err(e) = self.settings.save() {
                    log::warn!("Failed to save settings: {}", e);
                }
                if let Some(tray) = &self.tray {
                    tray.set_monitor_checked(new_state);
                    let status = if new_state { "ON" } else { "OFF" };
                    log::info!("Monitor toggled: {}", status);
                }
            }
            MenuAction::Stop => {
                log::info!("Stop requested");
                let _ = self.tts_tx.send(TTSCommand::Stop);
                // Stop animation immediately
                if let Some(tray) = &mut self.tray {
                    tray.stop_animation();
                }
            }
            MenuAction::ChangeVoice(voice) => {
                log::info!("Voice change requested: {}", voice);
                self.settings.current_voice = voice.clone();
                if let Err(e) = self.settings.save() {
                    log::warn!("Failed to save settings: {}", e);
                }
                let _ = self.tts_tx.send(TTSCommand::ChangeVoice { voice: voice.clone() });
                if let Some(tray) = &self.tray {
                    tray.set_voice_checked(&voice);
                }
            }
            MenuAction::Quit => {
                log::info!("Quit requested");
                self.shutdown.store(true, Ordering::SeqCst);
                let _ = self.tts_tx.send(TTSCommand::Shutdown);
            }
            MenuAction::Unknown => {}
        }
    }

    /// Check for TTS events and update UI
    fn check_tts_events(&mut self) {
        loop {
            match self.tts_event_rx.try_recv() {
                Ok(TTSEvent::ModelLoaded) => {
                    log::info!("Model loaded, ready for TTS");
                    self.model_loaded = true;
                    if let Some(tray) = &self.tray {
                        tray.set_tooltip("Pocket-Tray TTS - Ready");
                    }
                }
                Ok(TTSEvent::StartedSpeaking) => {
                    log::info!("Started speaking - starting animation");
                    if let Some(tray) = &mut self.tray {
                        tray.set_tooltip("Pocket-Tray TTS - Speaking...");
                        tray.start_animation();
                        self.last_animation_tick = Instant::now();
                    }
                }
                Ok(TTSEvent::FinishedSpeaking) => {
                    log::info!("Finished speaking - stopping animation");
                    if let Some(tray) = &mut self.tray {
                        tray.set_tooltip("Pocket-Tray TTS - Ready");
                        tray.stop_animation();
                    }
                }
                Ok(TTSEvent::Error(e)) => {
                    log::error!("TTS error: {}", e);
                    if let Some(tray) = &mut self.tray {
                        tray.set_tooltip(&format!("Pocket-Tray TTS - Error: {}", e));
                        tray.stop_animation();
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::error!("TTS event channel disconnected");
                    break;
                }
            }
        }
    }

    /// Update animation if needed
    fn tick_animation(&mut self) {
        if let Some(tray) = &mut self.tray {
            if tray.is_animating() {
                let now = Instant::now();
                if now.duration_since(self.last_animation_tick) >= ANIMATION_INTERVAL {
                    tray.tick_animation();
                    self.last_animation_tick = now;
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Create tray icon when the application is ready
        if self.tray.is_none() {
            match TrayManager::new(self.settings.monitor_enabled, &self.settings.current_voice) {
                Ok(tray) => {
                    tray.set_tooltip("Pocket-Tray TTS - Loading model...");
                    self.tray = Some(tray);
                    log::info!("Tray icon created");
                }
                Err(e) => {
                    log::error!("Failed to create tray icon: {}", e);
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
        // We don't have any windows, but need to implement this
        if self.shutdown.load(Ordering::SeqCst) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Process menu events
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            self.handle_menu_event(&event);
        }

        // Check for TTS events
        self.check_tts_events();

        // Tick animation if active
        self.tick_animation();

        // Check for shutdown
        if self.shutdown.load(Ordering::SeqCst) {
            event_loop.exit();
        }

        // Sleep a bit to avoid busy-waiting when not animating
        if let Some(tray) = &self.tray {
            if !tray.is_animating() {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
