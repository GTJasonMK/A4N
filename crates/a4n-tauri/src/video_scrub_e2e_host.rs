use std::collections::HashMap;

use base64::Engine as _;
use serde::Serialize;

#[derive(Serialize)]
struct VideoPlaybackData {
    mime_type: &'static str,
    data_base64: String,
    scrub_frames: Option<VideoScrubFramesData>,
}

#[derive(Serialize)]
struct VideoScrubFramesData {
    mime_type: &'static str,
    data_base64: String,
    frames_per_second: u32,
}

#[tauri::command]
fn get_all_frontend_state() -> HashMap<&'static str, &'static str> {
    HashMap::from([("a4n:current_user_id", "1")])
}

#[tauri::command]
fn set_frontend_state_many(entries: HashMap<String, String>) {
    drop(entries);
}

#[tauri::command]
fn remove_frontend_state_many(keys: Vec<String>) {
    drop(keys);
}

#[tauri::command]
fn load_video_scrub_playback_data(
    user_id: i64,
    video_id: i64,
    include_audio: bool,
) -> Result<VideoPlaybackData, String> {
    if user_id != 1 || !matches!(video_id, 9001 | 9002) || include_audio {
        return Err(format!(
            "unexpected scrub fixture request: user={user_id}, video={video_id}, include_audio={include_audio}"
        ));
    }
    let path = std::env::var("A4N_VIDEO_SCRUB_FIXTURE_PATH")
        .map_err(|_| "A4N_VIDEO_SCRUB_FIXTURE_PATH is required".to_string())?;
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("failed to read scrub fixture {path}: {error}"))?;
    let frames_path = std::env::var("A4N_VIDEO_SCRUB_FRAMES_FIXTURE_PATH")
        .map_err(|_| "A4N_VIDEO_SCRUB_FRAMES_FIXTURE_PATH is required".to_string())?;
    let frame_bytes = std::fs::read(&frames_path)
        .map_err(|error| format!("failed to read scrub frame fixture {frames_path}: {error}"))?;
    Ok(VideoPlaybackData {
        mime_type: "video/mp4",
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        scrub_frames: Some(VideoScrubFramesData {
            mime_type: "image/jpeg",
            data_base64: base64::engine::general_purpose::STANDARD.encode(frame_bytes),
            frames_per_second: 12,
        }),
    })
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .ok_or(tauri::Error::WindowNotFound)?;
            let window = tauri::WebviewWindowBuilder::from_config(app.handle(), config)?.build()?;
            window.navigate(
                "http://localhost:5173/e2e/video-scrub/"
                    .parse()
                    .map_err(tauri::Error::InvalidUrl)?,
            )?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_frontend_state,
            set_frontend_state_many,
            remove_frontend_state_many,
            load_video_scrub_playback_data,
        ])
        .run(tauri::generate_context!())
        .expect("video scrub E2E host failed");
}
