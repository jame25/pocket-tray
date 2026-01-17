//! Icon generation and animation for the system tray
//!
//! Creates a 3-vertical-bars icon design that animates during speech playback.

use image::{Rgba, RgbaImage};
use tray_icon::Icon;

/// DodgerBlue color (#1E90FF)
const ICON_COLOR: Rgba<u8> = Rgba([30, 144, 255, 255]);

/// Icon dimensions
const ICON_SIZE: u32 = 16;

/// Number of animation frames
const FRAME_COUNT: usize = 8;

/// X positions for the 3 vertical lines (evenly spaced)
const LINE_X_POSITIONS: [u32; 3] = [3, 7, 11];

/// Line width in pixels
const LINE_WIDTH: u32 = 2;

/// Minimum line height (pixels)
const MIN_HEIGHT: f64 = 4.0;

/// Maximum line height (pixels)
const MAX_HEIGHT: f64 = 10.0;

/// Static line heights for the non-animated icon
const STATIC_HEIGHTS: [u32; 3] = [6, 10, 8];

/// Generate the static (non-animated) tray icon
pub fn create_static_icon() -> anyhow::Result<Icon> {
    let mut img = RgbaImage::new(ICON_SIZE, ICON_SIZE);

    // Draw 3 vertical lines with static heights
    for (i, &x) in LINE_X_POSITIONS.iter().enumerate() {
        let height = STATIC_HEIGHTS[i];
        draw_vertical_line(&mut img, x, height);
    }

    image_to_icon(&img)
}

/// Generate all animation frames (8 frames for smooth sine wave animation)
pub fn create_animation_frames() -> anyhow::Result<Vec<Icon>> {
    let mut frames = Vec::with_capacity(FRAME_COUNT);

    for frame_index in 0..FRAME_COUNT {
        let img = create_animation_frame(frame_index);
        frames.push(image_to_icon(&img)?);
    }

    Ok(frames)
}

/// Create a single animation frame
fn create_animation_frame(frame_index: usize) -> RgbaImage {
    let mut img = RgbaImage::new(ICON_SIZE, ICON_SIZE);

    let frame_progress = frame_index as f64 / FRAME_COUNT as f64;

    for (line_index, &x) in LINE_X_POSITIONS.iter().enumerate() {
        // Calculate sine wave phase for this line
        // Each line has a 0.33 phase offset from the previous one
        let phase = (frame_progress + line_index as f64 * 0.33) * std::f64::consts::PI * 2.0;
        let wave_value = phase.sin();

        // Calculate height based on sine wave
        let height_range = (MAX_HEIGHT - MIN_HEIGHT) / 2.0;
        let center_height = MIN_HEIGHT + height_range;
        let line_height = (center_height + wave_value * height_range).round() as u32;

        draw_vertical_line(&mut img, x, line_height);
    }

    img
}

/// Draw a vertical line centered on the icon
fn draw_vertical_line(img: &mut RgbaImage, x: u32, height: u32) {
    let center_y = ICON_SIZE / 2;
    let half_height = height / 2;

    let y_start = center_y.saturating_sub(half_height);
    let y_end = (center_y + half_height).min(ICON_SIZE - 1);

    // Draw with rounded caps by filling multiple columns for line width
    for dx in 0..LINE_WIDTH {
        let px = x + dx;
        if px >= ICON_SIZE {
            continue;
        }

        for y in y_start..=y_end {
            img.put_pixel(px, y, ICON_COLOR);
        }

        // Round the caps by adding pixels at ends
        if y_start > 0 {
            // Top cap
            let alpha = 180u8; // Slightly transparent for anti-aliasing effect
            img.put_pixel(px, y_start.saturating_sub(1), Rgba([30, 144, 255, alpha]));
        }
        if y_end < ICON_SIZE - 1 {
            // Bottom cap
            let alpha = 180u8;
            img.put_pixel(px, y_end + 1, Rgba([30, 144, 255, alpha]));
        }
    }
}

/// Convert an RgbaImage to a tray Icon
fn image_to_icon(img: &RgbaImage) -> anyhow::Result<Icon> {
    let (width, height) = img.dimensions();
    let rgba_data = img.as_raw().to_vec();
    let icon = Icon::from_rgba(rgba_data, width, height)?;
    Ok(icon)
}

/// Animation state manager
pub struct IconAnimator {
    static_icon: Icon,
    animation_frames: Vec<Icon>,
    current_frame: usize,
    is_animating: bool,
}

impl IconAnimator {
    /// Create a new icon animator
    pub fn new() -> anyhow::Result<Self> {
        let static_icon = create_static_icon()?;
        let animation_frames = create_animation_frames()?;

        Ok(Self {
            static_icon,
            animation_frames,
            current_frame: 0,
            is_animating: false,
        })
    }

    /// Get the static (non-animated) icon
    pub fn static_icon(&self) -> &Icon {
        &self.static_icon
    }

    /// Start the animation
    pub fn start_animation(&mut self) {
        self.is_animating = true;
        self.current_frame = 0;
    }

    /// Stop the animation and return to static icon
    pub fn stop_animation(&mut self) {
        self.is_animating = false;
        self.current_frame = 0;
    }

    /// Check if animation is currently running
    pub fn is_animating(&self) -> bool {
        self.is_animating
    }

    /// Advance to the next animation frame and return it
    /// Returns None if not animating
    pub fn next_frame(&mut self) -> Option<&Icon> {
        if !self.is_animating || self.animation_frames.is_empty() {
            return None;
        }

        self.current_frame = (self.current_frame + 1) % self.animation_frames.len();
        Some(&self.animation_frames[self.current_frame])
    }

    /// Get the current icon (animated frame if animating, static otherwise)
    #[allow(dead_code)]
    pub fn current_icon(&self) -> &Icon {
        if self.is_animating && !self.animation_frames.is_empty() {
            &self.animation_frames[self.current_frame]
        } else {
            &self.static_icon
        }
    }
}
