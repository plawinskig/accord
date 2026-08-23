use crate::constants::ATTACHMENTS_DIR;
use crate::AppState;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::http::{Request, Response};
use tauri::{Manager, UriSchemeContext, Wry};

/// Handles requests to the custom `accord://` protocol. The frontend uses it
/// to load attachment files straight from disk (both files copied/moved into
/// the workspace, and files that are only linked in place), bypassing the
/// default asset protocol.
pub fn handle_accord_protocol(ctx: UriSchemeContext<'_, Wry>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let app = ctx.app_handle();
    let state = app.state::<AppState>();
    let ws_guard = state.workspace_path.lock().unwrap();

    let Some(workspace) = ws_guard.as_ref() else {
        return not_found();
    };

    let uri_str = request.uri().to_string();
    println!("[Accord Protocol] A request was received: {}", uri_str);

    let Some(file_path) = resolve_file_path(workspace, &uri_str) else {
        println!("[Accord Protocol] Unrecognized URL format");
        return bad_request();
    };

    println!("[Accord Protocol] Looking for the file on disk: {:?}", file_path);

    match fs::read(&file_path) {
        Ok(data) => {
            let mime_type = mime_type_for(&file_path);
            println!("[Accord Protocol] Success! Sending as {}", mime_type);
            Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-Type", mime_type)
                .body(data)
                .unwrap()
        }
        Err(_) => {
            println!("[Accord Protocol] ERROR: File not found on disk!");
            not_found()
        }
    }
}

/// Turns an `accord://local/...` or `accord://link/...` URL into an absolute
/// path on disk. `local` paths are resolved relative to the workspace's
/// attachments folder; `link` paths are used as-is (they are files the user
/// chose to reference in place rather than copy). Also handles the case
/// where Tauri rewrites the authority so the request shows up as `/local/...`
/// instead of `accord://local/...`.
fn resolve_file_path(workspace: &str, uri_str: &str) -> Option<PathBuf> {
    if let Some((_, local_part)) = uri_str.split_once("accord://local/") {
        let decoded = urlencoding::decode(local_part).unwrap_or_default();
        Some(Path::new(workspace).join(ATTACHMENTS_DIR).join(decoded.into_owned()))
    } else if let Some((_, link_part)) = uri_str.split_once("accord://link/") {
        let decoded = urlencoding::decode(link_part).unwrap_or_default();
        Some(PathBuf::from(decoded.into_owned()))
    } else if let Some((_, local_part)) = uri_str.split_once("/local/") {
        let decoded = urlencoding::decode(local_part).unwrap_or_default();
        Some(Path::new(workspace).join(ATTACHMENTS_DIR).join(decoded.into_owned()))
    } else {
        None
    }
}

/// Maps a file extension to the MIME type the frontend needs to render it
/// (an `<img>`, a PDF viewer, etc). Anything unrecognized is sent as a
/// generic binary stream, which is safe but won't render inline.
fn mime_type_for(path: &Path) -> &'static str {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match extension.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder().status(404).body(vec![]).unwrap()
}

fn bad_request() -> Response<Vec<u8>> {
    Response::builder().status(400).body(vec![]).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_local_path_inside_attachments_dir() {
        let path = resolve_file_path("/home/user/workspace", "accord://local/abc-123.png").unwrap();
        assert_eq!(path, PathBuf::from("/home/user/workspace/attachments/abc-123.png"));
    }

    #[test]
    fn resolves_link_path_as_is() {
        let path = resolve_file_path("/home/user/workspace", "accord://link/%2Fhome%2Fuser%2Fdoc.pdf").unwrap();
        assert_eq!(path, PathBuf::from("/home/user/doc.pdf"));
    }

    #[test]
    fn falls_back_to_bare_local_segment() {
        let path = resolve_file_path("/home/user/workspace", "http://accord.localhost/local/abc.png").unwrap();
        assert_eq!(path, PathBuf::from("/home/user/workspace/attachments/abc.png"));
    }

    #[test]
    fn returns_none_for_unrecognized_uri() {
        assert!(resolve_file_path("/home/user/workspace", "accord://unknown/abc.png").is_none());
    }

    #[test]
    fn mime_type_matches_known_extensions() {
        assert_eq!(mime_type_for(Path::new("photo.PNG")), "image/png");
        assert_eq!(mime_type_for(Path::new("doc.pdf")), "application/pdf");
        assert_eq!(mime_type_for(Path::new("archive.zip")), "application/octet-stream");
    }
}
