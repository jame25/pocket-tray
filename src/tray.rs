//! System tray icon and menu management

use crate::icon::IconAnimator;
use crate::settings::VOICES;
use anyhow::Result;
use muda::{accelerator::Accelerator, CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{TrayIcon, TrayIconBuilder};

/// Menu item IDs
pub mod menu_ids {
    use muda::MenuId;

    pub fn monitor() -> MenuId {
        MenuId::new("monitor")
    }

    pub fn stop() -> MenuId {
        MenuId::new("stop")
    }

    pub fn quit() -> MenuId {
        MenuId::new("quit")
    }

    pub fn voice(name: &str) -> MenuId {
        MenuId::new(format!("voice_{}", name))
    }

    pub fn voice_name_from_id(id: &MenuId) -> Option<String> {
        let id_str = id.as_ref();
        if id_str.starts_with("voice_") {
            Some(id_str.strip_prefix("voice_").unwrap().to_string())
        } else {
            None
        }
    }
}

/// Tray icon and menu manager
pub struct TrayManager {
    tray_icon: TrayIcon,
    monitor_item: CheckMenuItem,
    voice_items: Vec<CheckMenuItem>,
    animator: IconAnimator,
}

impl TrayManager {
    /// Create the tray icon and menu
    pub fn new(monitor_enabled: bool, current_voice: &str) -> Result<Self> {
        // Create icon animator
        let animator = IconAnimator::new()?;
        let icon = animator.static_icon().clone();

        // Build menu
        let menu = Menu::new();

        // Monitor toggle
        let monitor_item = CheckMenuItem::with_id(
            menu_ids::monitor(),
            "Monitoring",
            true,
            monitor_enabled,
            None::<Accelerator>,
        );

        // Stop button
        let stop_item = MenuItem::with_id(menu_ids::stop(), "Stop", true, None::<Accelerator>);

        // Voices submenu
        let voices_menu = Submenu::new("Voices", true);
        let mut voice_items = Vec::new();
        for &name in VOICES {
            let checked = name == current_voice;
            let item = CheckMenuItem::with_id(
                menu_ids::voice(name),
                name,
                true,
                checked,
                None::<Accelerator>,
            );
            voices_menu.append(&item)?;
            voice_items.push(item);
        }

        // Quit
        let quit_item = MenuItem::with_id(menu_ids::quit(), "Quit", true, None::<Accelerator>);

        // Assemble menu
        menu.append(&monitor_item)?;
        menu.append(&stop_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&voices_menu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        // Create tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Pocket-Tray TTS")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            tray_icon,
            monitor_item,
            voice_items,
            animator,
        })
    }

    /// Update the monitor checkbox state
    pub fn set_monitor_checked(&self, checked: bool) {
        self.monitor_item.set_checked(checked);
    }

    /// Update which voice is selected
    pub fn set_voice_checked(&self, voice_name: &str) {
        for item in &self.voice_items {
            let is_selected = item.text() == voice_name;
            item.set_checked(is_selected);
        }
    }

    /// Update the tooltip
    pub fn set_tooltip(&self, tooltip: &str) {
        let _ = self.tray_icon.set_tooltip(Some(tooltip));
    }

    /// Start the icon animation (call when speaking starts)
    pub fn start_animation(&mut self) {
        self.animator.start_animation();
        // Set the first animation frame
        if let Some(frame) = self.animator.next_frame() {
            let _ = self.tray_icon.set_icon(Some(frame.clone()));
        }
    }

    /// Stop the icon animation (call when speaking stops)
    pub fn stop_animation(&mut self) {
        self.animator.stop_animation();
        // Restore the static icon
        let _ = self.tray_icon.set_icon(Some(self.animator.static_icon().clone()));
    }

    /// Advance to the next animation frame (call every ~120ms when animating)
    /// Returns true if animation is active, false otherwise
    pub fn tick_animation(&mut self) -> bool {
        if !self.animator.is_animating() {
            return false;
        }

        if let Some(frame) = self.animator.next_frame() {
            let _ = self.tray_icon.set_icon(Some(frame.clone()));
            true
        } else {
            false
        }
    }

    /// Check if animation is currently running
    pub fn is_animating(&self) -> bool {
        self.animator.is_animating()
    }
}

/// Menu event handler results
pub enum MenuAction {
    ToggleMonitor,
    Stop,
    ChangeVoice(String),
    Quit,
    Unknown,
}

/// Process a menu event and return the corresponding action
pub fn process_menu_event(event: &MenuEvent) -> MenuAction {
    let id = event.id();

    if id == &menu_ids::monitor() {
        MenuAction::ToggleMonitor
    } else if id == &menu_ids::stop() {
        MenuAction::Stop
    } else if id == &menu_ids::quit() {
        MenuAction::Quit
    } else if let Some(voice) = menu_ids::voice_name_from_id(id) {
        MenuAction::ChangeVoice(voice)
    } else {
        MenuAction::Unknown
    }
}
