use super::*;
use crate::actions::video_generation::provider_request::{
    ProviderGenerationWorkflow, prompt_provider_workflow, reference_ids_with_storyboard_image,
};
use a4n_prompt::tera_engine::TeraPromptEngine;
use chrono::Utc;
use std::path::Path;

const LONG_VISUAL: &str = "[主体定义与素材绑定]\n首帧输入图片定义小宦官站在昏暗殿内中央偏左的开头画面，尾帧输入图片定义他靠近暗廊入口但不越过的结尾画面；参考图只用于保持内侍服饰、殿内柱影、暗廊入口和烛光空间关系一致。\n\n[片段内容以及前因后果]\n小宦官从殿内中央偏左位置承接上一状态，听见右后方暗廊方向传来细微异响后压住恐惧回望，结尾停在暗廊入口前并保持警觉。\n\n[镜头分镜]\n镜头 1：平视中景从殿内中央偏左的首帧承接，小宦官停住听向右后方暗角，肩膀轻微收紧，柱影和地面烛光反射保持清晰。\n镜头 2：镜头维持同一轴线轻微手持漂移，他原地缓慢后缩半步，靠近暗廊入口但不越过，最后稳定为回望黑暗走廊的戒备姿态。\n\n[画面风格描述]\n电影感写实，右侧暖烛光与暗廊冷阴影对比，平视中景，柱影和地面烛光反射作为固定参照物，整体保持压抑悬疑的低饱和色调。\n\n[连续性与结尾状态]\n小宦官脸部、身形和内侍服饰不变，暗廊始终位于画面右后方，殿门余光、柱影和地面烛光反射位置不变；结尾人物靠近暗廊入口但仍留在入口前，回望黑暗走廊。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、乱码文字、换脸、换装、跳轴、空间跳变、光影方向突变或把参考图画成拼贴。";
const LONG_MOTION: &str = "[运镜与动作]\n镜头保持平视中景和轻微手持漂移但不跳轴，小宦官先停在殿内中央偏左看向暗角，然后原地轻微后缩，缓慢回头望向右后方暗廊，肩膀收紧并压住恐惧，动作节奏缓慢且稳定。\n\n[对白与表演]\n小宦官压低声音说“谁在那儿”，口型短促，眼神避开又重新看向暗廊，表情保持害怕但努力镇定，停顿后屏住呼吸。";
const LONG_EN_VISUAL: &str = "[Subject And Asset Binding]\nThe first input frame defines the young palace attendant standing near the center-left of a dim hall, while the ending frame defines him close to the dark corridor entrance without crossing it; reference images only preserve costume, face, pillar shadows, corridor placement, and candlelight direction.\n\n[Segment Context]\nThe young palace attendant continues from the previous tense pause, hears a faint disturbance from the right rear corridor, suppresses fear, turns back, and ends near the corridor entrance while staying alert and physically restrained.\n\n[Shot Storyboard]\nShot 1: an eye-level medium shot carries over from the center-left hall position; the character holds still, shoulders tightening as his gaze shifts toward the right rear darkness while the pillars and floor reflections stay readable.\nShot 2: the camera keeps the same axis with a restrained drift; he subtly steps back in place, approaches the corridor entrance without crossing it, and stabilizes in a guarded look toward the dark passage.\n\n[Visual Style]\nCinematic realism with warm candlelight from screen right contrasting against the cool darkness of the corridor, eye-level medium framing, readable pillar shadows, floor reflections, muted color, and a grounded suspenseful atmosphere.\n\n[Continuity And End State]\nThe character face, body shape, attendant costume, hall layout, corridor direction, candlelight direction, and relative positions of pillars and floor reflections remain unchanged; the final frame holds him before the corridor entrance looking back into darkness.\n\n[Constraints And Avoidances]\nNo background music, subtitles, watermarks, garbled text, face swap, costume change, axis jump, spatial jump, lighting direction jump, collage layout, extra limbs, or unreadable text.";
const LONG_EN_MOTION: &str = "[Camera And Action]\nUse an eye-level medium shot with a restrained slow drift that never crosses the axis. The young palace attendant first pauses at the center-left of the hall, then makes a small backward shift, slowly turns toward the right rear corridor, tightens his shoulders, and controls his fear while the movement remains slow and readable.\n\n[Dialogue And Performance]\nThe character quietly says \"谁在那儿\" in the original Chinese line, with short lip movement, shallow breathing, guarded eyes, and a frightened but controlled expression; the spoken line must not become subtitles.";

fn prompt_engine() -> TeraPromptEngine {
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
}

fn provider_prompt_image_binding(
    prompt: &chapter_video_prompt::Model,
    boundary_frame_ids: &BoundaryFrameImageIds,
) -> ProviderPromptImageBinding {
    ProviderPromptImageBinding {
        has_first_frame_image: boundary_frame_ids.first.is_some(),
        has_last_frame_image: boundary_frame_ids.last.is_some(),
        reference_image_count: media::reference_image_ids(prompt.reference_image_ids_json.as_ref())
            .expect("test prompt reference ids should be valid")
            .len(),
        has_storyboard_reference_image: false,
    }
}

fn build_video_generation_provider_prompt(
    prompt: &chapter_video_prompt::Model,
    boundary_frame_ids: &BoundaryFrameImageIds,
) -> Result<String, String> {
    let engine = prompt_engine();
    super::build_video_generation_provider_prompt_with_binding(
        &engine,
        prompt,
        provider_prompt_image_binding(prompt, boundary_frame_ids),
    )
}

fn build_video_generation_provider_preview_prompt(
    prompt: &chapter_video_prompt::Model,
    boundary_frame_ids: &BoundaryFrameImageIds,
) -> Result<String, String> {
    let engine = prompt_engine();
    super::build_video_generation_provider_preview_prompt_with_binding(
        &engine,
        prompt,
        provider_prompt_image_binding(prompt, boundary_frame_ids),
    )
}

fn prompt_with_metadata() -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id: 7,
        project_id: 1,
        chapter_id: 2,
        style: Some("电影感写实".to_string()),
        prompt_type: "video_prompt".to_string(),
        workflow_mode: "boundary_frames".to_string(),
        shot_number: 1,
        title: Some("回望暗廊".to_string()),
        scene_summary: Some("小宦官回望暗处，压住恐惧".to_string()),
        visual_prompt: LONG_VISUAL.to_string(),
        motion_prompt: Some(LONG_MOTION.to_string()),
        negative_prompt: Some(
            "低清晰度、画面模糊、人体结构扭曲、多余肢体、面部不一致、空间跳变、光影不连续"
                .to_string(),
        ),
        duration_secs: 4,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: Some(json!([11])),
        status: "pending".to_string(),
        generated_video_id: None,
        metadata_json: Some(json!({
            "shot": {
                "title": "回望暗廊",
                "scene_summary": "小宦官回望暗处，压住恐惧",
                "blocking_plan": {"start": "殿内中央偏左", "movement": "原地后缩回望", "end": "靠近暗廊入口但不越过"},
                "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "continuity": "继承首尾帧"},
                "emotion_action_beats": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "肩膀收紧并回头"}],
                "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微手持漂移", "axis_rule": "暗廊保持右后方", "lens_hint": "标准镜头"},
                "temporal_beats": [{"start_sec": 0, "end_sec": 4, "camera": "中景轻微漂移", "subject_action": "回头并靠近暗廊入口", "continuity_target": "抵达尾帧"}],
                "reference_binding": {"bindings": [{"image_id": 11, "entity": "小宦官", "role": "character", "usage": "保持脸和服装一致", "must_keep": "同一人物"}], "coverage_check": "参考图 11 已绑定"},
                "continuity_check": {"blocking": "连续", "lighting": "连续", "emotion": "已动作化", "references": "已覆盖", "revision": "无需修订"}
            },
            "dialogue_beats": [
                {"speaker": "小宦官", "line": "谁在那儿", "delivery": "压低声音试探", "visible_action": "嘴唇微动并回头", "listener_reaction": "暗廊无人回应", "timing": "中段"}
            ],
            "start_boundary_frame": {
                "title": "殿内首帧",
                "scene_summary": "小宦官站在殿内中央偏左",
                "frame_prompt": "小宦官站在昏暗殿内中央偏左，看向右后方暗角，烛光从画面右侧斜照。",
                "blocking_state": {"characters": [{"name": "小宦官", "position": "中央偏左"}]},
                "lighting_state": {"key_light": "烛光", "direction": "画面右侧"}
            },
            "end_boundary_frame": {
                "title": "暗廊尾帧",
                "scene_summary": "小宦官靠近暗廊入口并回望",
                "frame_prompt": "小宦官靠近暗廊入口但没有越过，回望右后方黑暗走廊。",
                "blocking_state": {"characters": [{"name": "小宦官", "position": "暗廊入口前"}]},
                "lighting_state": {"key_light": "烛光", "direction": "画面右侧"}
            }
        })),
        created_at: now,
        updated_at: now,
    }
}

fn reference_media_video_with_metadata(metadata_json: Option<Value>) -> generated_video::Model {
    let now = Utc::now().naive_utc();
    generated_video::Model {
        id: 42,
        project_id: 1,
        chapter_id: Some(2),
        video_type: "reference_media_video".to_string(),
        workflow_mode: Some("reference_media".to_string()),
        file_path: None,
        thumbnail_path: None,
        prompt: "这是一段足够长的参考媒体视频提示词，用于描述人物外观、场景结构、镜头调度、动作节奏、光影方向、情绪变化和画面风格；其中明确使用 @image_file_1 作为主角造型参考，并使用 @video_file_1 作为动作节奏参考。".to_string(),
        negative_prompt: None,
        model: "seedance-2-0-pro".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 8.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json,
        is_favorite: false,
        created_at: now,
        updated_at: now,
    }
}

fn video_config(backend: &str) -> video_gen_config::Model {
    let now = Utc::now().naive_utc();
    video_gen_config::Model {
        id: 9,
        name: "test video config".to_string(),
        backend: backend.to_string(),
        base_url: Some("https://example.test".to_string()),
        api_key_encrypted: Some("test-key".to_string()),
        model: "test-video-model".to_string(),
        max_concurrent: 1,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 4,
        min_duration_secs: 1,
        max_duration_secs: 60,
        duration_mode: "range".to_string(),
        supported_durations_json: None,
        default_fps: 24,
        provider_options_json: None,
        is_default: true,
        created_at: now,
        updated_at: now,
    }
}

#[cfg(feature = "embedded-media-providers")]
fn provider_preview_context(backend: &str) -> ChapterVideoProviderRequestContext {
    ChapterVideoProviderRequestContext {
        prompt: prompt_with_metadata(),
        cfg: video_config(backend),
        request: VideoGenerationRequest {
            prompt: "导演提示词正文".to_string(),
            negative_prompt: Some("低清晰度".to_string()),
            width: 1280,
            height: 720,
            duration_secs: 4,
            fps: 24,
            model: "test-video-model".to_string(),
            first_frame_image_url: Some("https://example.invalid/a4n-preview/first".into()),
            last_frame_image_url: Some("https://example.invalid/a4n-preview/last".into()),
            reference_image_urls: vec!["https://example.invalid/a4n-preview/ref".into()],
            reference_media: None,
        },
        prompt_reference_image_ids: Some(json!([11])),
        provider_reference_image_ids: Some(json!([11])),
        prompt_source: None,
        workflow_mode: "boundary_frames".to_string(),
        source_prompt_type: Some("frame_segment_plan".to_string()),
        boundary_frame_ids: BoundaryFrameImageIds {
            first: Some(21),
            last: Some(22),
        },
        prompt_reference_image_count: 1,
        provider_reference_image_count: 1,
        first_frame_image_url_available: true,
        last_frame_image_url_available: true,
        provider_input_note: None,
    }
}

mod canvas;
mod prompt_rendering;
mod provider_preview;
mod reference_media;
mod results;
mod rules;
mod snapshot;
