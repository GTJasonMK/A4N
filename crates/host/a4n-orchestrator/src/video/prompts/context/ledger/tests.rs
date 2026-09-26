use std::collections::HashMap;
use std::future::Future;
use std::path::Path;

use a4n_prompt::tera_engine::TeraPromptEngine;
use serde_json::{Value, json};

use super::{ChapterVideoSourceContext, build_final_video_context_manifest};

const PROJECT_ID: i64 = 1;

trait FutureResultExt<T, E>: Future<Output = Result<T, E>> + Sized {
    fn expect(self, message: &str) -> T
    where
        E: std::fmt::Debug,
    {
        futures::executor::block_on(self).expect(message)
    }
}

impl<F, T, E> FutureResultExt<T, E> for F where F: Future<Output = Result<T, E>> + Sized {}

fn prompt_renderer() -> TeraPromptEngine {
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts");
    TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
}

fn contains_json_key(value: &Value, target: &str) -> bool {
    match value {
        Value::Array(items) => items.iter().any(|item| contains_json_key(item, target)),
        Value::Object(object) => {
            object.contains_key(target)
                || object
                    .values()
                    .any(|value| contains_json_key(value, target))
        }
        _ => false,
    }
}

#[test]
fn final_video_context_manifest_excludes_internal_source_fields() {
    let engine = prompt_renderer();
    let analysis = json!({
        "scene": "教室窗边对白",
        "source_span_ids": ["ch1:s1:f1"],
        "nested": {
            "source_span_id": "ch1:s1:f1",
            "dialogue_beats": [{
                "speaker": "林清辞",
                "line": "等等",
                "delivery": "低声",
                "visible_action": "嘴唇微动",
                "listener_reaction": "对方停步",
                "timing": "中段",
                "source_span_ids": ["ch1:s1:f1"]
            }]
        }
    });
    let source = ChapterVideoSourceContext {
        content: "林清辞站在窗边低声说：“等等。”",
        summary: Some("窗边对白"),
        analysis: Some(&analysis),
    };
    let manifest = build_final_video_context_manifest(
        &engine,
        PROJECT_ID,
        &HashMap::new(),
        &source,
        &["chapter_video_prompt"],
        "请根据以下 JSON 输入生成视频提示词".to_string(),
    )
    .expect("final video context manifest should build");
    let manifest_json = serde_json::to_value(&manifest).expect("manifest should serialize");
    let manifest_text = serde_json::to_string(&manifest_json).expect("manifest text");

    assert!(
        manifest
            .packs
            .iter()
            .all(|pack| !pack.pack_kind.starts_with("video_previous_output:")),
        "final video manifest must not include raw previous-output packs"
    );
    for key in [
        "source_span_id",
        "source_span_ids",
        "key_source_span_ids",
        "source_span_range",
        "reference_image_ids",
    ] {
        assert!(
            !contains_json_key(&manifest_json, key),
            "final video context manifest must not expose internal key {key}"
        );
        assert!(
            !manifest_text.contains(key),
            "final video context manifest text must not expose internal field name {key}"
        );
    }
    assert!(
        manifest
            .packs
            .iter()
            .any(|pack| pack.pack_kind == "video_chapter_source"),
        "final video context should keep public chapter source for dialogue extraction"
    );
    assert!(
        manifest.packs[0].source_refs_json.as_ref().unwrap()["prompt_dependencies"].is_array(),
        "模板依赖保存在非模型元数据中，仍参与断点指纹"
    );
    let model_content = serde_json::to_string(&manifest.packs[0].variants).unwrap();
    assert!(!model_content.contains("sha256"));
    assert!(!model_content.contains("analysis"));
    assert_eq!(
        manifest.packs[0].variants.len(),
        1,
        "需要正文时不能降级成 hash"
    );
}
