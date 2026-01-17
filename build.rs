//! Build script for Pocket-Tray
//!
//! Generates the application icon and embeds it along with the manifest
//! into the Windows executable.

#[cfg(windows)]
use std::io::BufWriter;

fn main() {
    #[cfg(windows)]
    {
        // Generate the icon file
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
        let ico_path = std::path::Path::new(&out_dir).join("icon.ico");

        if let Err(e) = generate_icon(&ico_path) {
            eprintln!("Warning: Failed to generate icon: {}", e);
        } else {
            // Embed icon and manifest
            let mut res = winres::WindowsResource::new();
            res.set_icon(ico_path.to_str().unwrap());

            // Set application manifest for DPI awareness and visual styles
            res.set_manifest(
                r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="1.0.0.0"
    processorArchitecture="*"
    name="PocketTray"
    type="win32"
  />
  <description>Pocket TTS System Tray Application</description>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">permonitorv2,permonitor</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#,
            );

            if let Err(e) = res.compile() {
                eprintln!("Warning: Failed to compile Windows resources: {}", e);
            }
        }
    }
}

/// Generate an ICO file with multiple sizes
#[cfg(windows)]
fn generate_icon(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
    use image::{Rgba, RgbaImage};

    let sizes = [16, 32, 48, 256];
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    for &size in &sizes {
        let img = generate_icon_image(size);
        let rgba_data = img.into_raw();

        let icon_image = IconImage::from_rgba_data(size, size, rgba_data);
        icon_dir.add_entry(IconDirEntry::encode(&icon_image)?);
    }

    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);
    icon_dir.write(writer)?;

    Ok(())
}

/// Generate a single icon image at the specified size
/// This replicates the design from src/icon.rs: 3 vertical bars in DodgerBlue
#[cfg(windows)]
fn generate_icon_image(size: u32) -> image::RgbaImage {
    use image::{Rgba, RgbaImage};

    // DodgerBlue color (#1E90FF)
    let icon_color: Rgba<u8> = Rgba([30, 144, 255, 255]);

    // Scale factor relative to 16x16 base
    let scale = size as f64 / 16.0;

    // Static heights at 16x16 scale (from icon.rs)
    let static_heights_base: [f64; 3] = [6.0, 10.0, 8.0];

    // Line X positions at 16x16 scale
    let line_x_base: [f64; 3] = [3.0, 7.0, 11.0];

    // Line width at 16x16 scale
    let line_width_base: f64 = 2.0;

    let mut img = RgbaImage::new(size, size);

    for i in 0..3 {
        let x = (line_x_base[i] * scale).round() as u32;
        let height = (static_heights_base[i] * scale).round() as u32;
        let line_width = (line_width_base * scale).round().max(1.0) as u32;

        draw_vertical_line(&mut img, x, height, line_width, size, icon_color);
    }

    img
}

/// Draw a vertical line centered on the icon
#[cfg(windows)]
fn draw_vertical_line(
    img: &mut image::RgbaImage,
    x: u32,
    height: u32,
    line_width: u32,
    icon_size: u32,
    color: image::Rgba<u8>,
) {
    let center_y = icon_size / 2;
    let half_height = height / 2;

    let y_start = center_y.saturating_sub(half_height);
    let y_end = (center_y + half_height).min(icon_size - 1);

    // Draw with line width
    for dx in 0..line_width {
        let px = x + dx;
        if px >= icon_size {
            continue;
        }

        for y in y_start..=y_end {
            img.put_pixel(px, y, color);
        }

        // Round the caps with slight transparency for anti-aliasing
        let alpha_color = image::Rgba([color[0], color[1], color[2], 180]);
        if y_start > 0 {
            img.put_pixel(px, y_start - 1, alpha_color);
        }
        if y_end < icon_size - 1 {
            img.put_pixel(px, y_end + 1, alpha_color);
        }
    }
}
