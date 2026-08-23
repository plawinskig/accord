use arboard::Clipboard;
use base64::{engine::general_purpose, Engine as _};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;

// Read image data directly from the system clipboard because WebKitGTK on Linux
// does not reliably expose it through the browser Clipboard API. Return a
// base64-encoded PNG, or None when the clipboard holds text or a file path.
#[tauri::command]
pub fn read_clipboard_image() -> Result<Option<String>, String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to open the clipboard: {e}"))?;

    let img = match clipboard.get_image() {
        Ok(img) => img,            // Handle the image returned by the native fallback
        Err(_) => return Ok(None), // Handle the regular paste event
    };

    let width = img.width as u32;
    let height = img.height as u32;

    // Encode arboard's raw RGBA8 pixels into a PNG in memory
    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, img.bytes.into_owned())
            .ok_or("Invalid image data from the clipboard")?;

    let mut png_bytes: Vec<u8> = Vec::new();
    buffer
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {e}"))?;

    Ok(Some(general_purpose::STANDARD.encode(png_bytes)))
}
