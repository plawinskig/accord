use arboard::Clipboard;
use base64::{engine::general_purpose, Engine as _};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;

// WebKitGTK on Linux does not reliably expose image data through the
// browser Clipboard API (DataTransfer.items stays empty for screenshots),
// so instead of relying on the frontend `paste` event we read the system
// clipboard directly here via `arboard`, which talks to GTK/X11/Wayland
// natively. Returns a base64-encoded PNG, or None if the clipboard
// currently holds no image (e.g. it holds text or a file path instead,
// in which case the frontend's existing text-based paste handling takes over).
#[tauri::command]
pub fn read_clipboard_image() -> Result<Option<String>, String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Nie udało się otworzyć schowka: {e}"))?;

    let img = match clipboard.get_image() {
        Ok(img) => img,            // handleKeydownPasteFallback
        Err(_) => return Ok(None), // handlePaste
    };

    let width = img.width as u32;
    let height = img.height as u32;

    // arboard returns raw RGBA8 pixels; encode them into a PNG in memory
    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, img.bytes.into_owned())
            .ok_or("Nieprawidłowe dane obrazu ze schowka")?;

    let mut png_bytes: Vec<u8> = Vec::new();
    buffer
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Nie udało się zakodować PNG: {e}"))?;

    Ok(Some(general_purpose::STANDARD.encode(png_bytes)))
}
