use std::collections::HashMap;

use a4n_api_contract::video::{VideoPlaybackData, VideoScrubFramesData};
use base64::Engine as _;

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
fn invoke_host_command(
    window: tauri::WebviewWindow,
    user_id: i64,
    command: String,
    args: serde_json::Value,
) -> Result<VideoPlaybackData, String> {
    let video_id = args.get("videoId").and_then(serde_json::Value::as_i64);
    if user_id != 1 || !matches!(video_id, Some(9001 | 9002)) {
        return Err(format!(
            "unexpected playback fixture request: user={user_id}, video={video_id:?}"
        ));
    }
    let include_frames = match command.as_str() {
        "load_video_scrub_playback_data" => {
            if window
                .url()
                .map_err(|error| error.to_string())?
                .query_pairs()
                .any(|(key, value)| key == "scenario" && value == "source-fallback")
            {
                return Err("scrub proxy fixture failure".to_string());
            }
            !args
                .get("includeAudio")
                .and_then(serde_json::Value::as_bool)
                .ok_or_else(|| "includeAudio must be explicit".to_string())?
        }
        "load_video_playback_data" => false,
        _ => return Err(format!("unexpected fixture command: {command}")),
    };
    let path = std::env::var("A4N_VIDEO_SCRUB_FIXTURE_PATH")
        .map_err(|_| "A4N_VIDEO_SCRUB_FIXTURE_PATH is required".to_string())?;
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("failed to read scrub fixture {path}: {error}"))?;
    let scrub_frames = if include_frames {
        let frames_path = std::env::var("A4N_VIDEO_SCRUB_FRAMES_FIXTURE_PATH")
            .map_err(|_| "A4N_VIDEO_SCRUB_FRAMES_FIXTURE_PATH is required".to_string())?;
        let frame_bytes = std::fs::read(&frames_path).map_err(|error| {
            format!("failed to read scrub frame fixture {frames_path}: {error}")
        })?;
        Some(VideoScrubFramesData {
            mime_type: "image/jpeg".to_string(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(frame_bytes),
            frames_per_second: 12,
        })
    } else {
        None
    };
    Ok(VideoPlaybackData {
        mime_type: "video/mp4".to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        scrub_frames,
    })
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let mut config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or(tauri::Error::WindowNotFound)?;
            // Let WebDriver size the window before it starts a media pipeline.
            config.url = tauri::WebviewUrl::External(
                "about:blank".parse().map_err(tauri::Error::InvalidUrl)?,
            );
            tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_frontend_state,
            set_frontend_state_many,
            remove_frontend_state_many,
            invoke_host_command,
        ])
        .run(tauri::generate_context!())
        .expect("video scrub E2E host failed");
}
