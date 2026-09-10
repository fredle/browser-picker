//! Profile photos: decode, scale, and crop to a circle.

use eframe::egui;
use std::path::Path;

/// Load a profile picture as a circular texture. Returns None if the file is
/// missing or undecodable, in which case the caller draws an initial instead.
pub fn load(ctx: &egui::Context, path: &Path, size: usize) -> Option<egui::TextureHandle> {
    let img = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let img = img.resize_exact(
        size as u32,
        size as u32,
        image::imageops::FilterType::Lanczos3,
    );
    let mut px = img.to_rgba8().into_raw();

    // Circular alpha mask, feathered by one pixel so the edge isn't jagged.
    let r = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - r;
            let dy = y as f32 + 0.5 - r;
            let coverage = (r - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let a = (y * size + x) * 4 + 3;
            px[a] = (px[a] as f32 * coverage) as u8;
        }
    }

    let image = egui::ColorImage::from_rgba_unmultiplied([size, size], &px);
    Some(ctx.load_texture(
        path.to_string_lossy().to_string(),
        image,
        egui::TextureOptions::LINEAR,
    ))
}
