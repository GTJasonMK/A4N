use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_core::models::chapter::chapter_manga_prompt;

use super::layout_rules::manga_page_generation_layout;

/// 将持久化的整页漫画 layout 投影为唯一对外 API 合同。
pub fn manga_page_prompt_summary(
    prompt: &chapter_manga_prompt::Model,
) -> Result<MangaPagePromptSummary, String> {
    let layout = manga_page_generation_layout(prompt)?;
    Ok(MangaPagePromptSummary {
        id: prompt.id,
        chapter_id: prompt.chapter_id,
        project_id: prompt.project_id,
        page_number: prompt.page_number,
        image_prompt: prompt.image_prompt.clone(),
        dialogue_text: prompt.dialogue_text.clone(),
        page_plan: layout.page_plan,
        drawing_prompt: layout.drawing_prompt,
        reference_bindings: layout.reference_bindings,
        source: layout.source,
        generated_image_id: prompt.generated_image_id,
        status: prompt.status.clone(),
        created_at: prompt.created_at.to_string(),
        updated_at: prompt.updated_at.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::chapter::chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE;
    use chrono::NaiveDateTime;
    use serde_json::{Value, json};

    fn page_prompt(layout: Option<Value>) -> chapter_manga_prompt::Model {
        let now = NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("valid timestamp");
        chapter_manga_prompt::Model {
            id: 11,
            chapter_id: 22,
            project_id: 33,
            page_number: 1,
            prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
            image_prompt: "整页提示词".to_string(),
            dialogue_text: Some("你好".to_string()),
            characters_in_frame: Some(json!(["主角"])),
            layout,
            generated_image_id: Some(44),
            status: "completed".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn valid_layout() -> Value {
        json!({
            "schema_version": "manga_page_generation_v2",
            "page_plan": {
                "page_number": 1,
                "beat_refs": [{
                    "scene_index": 1,
                    "shot_index": 1,
                    "beat_index": 1
                }],
                "role": "setup",
                "pacing": "medium",
                "importance": "normal",
                "reading_flow": "top_to_bottom_left_to_right",
                "page_goal": "推进场景",
                "visual_thesis": "角色对峙",
                "entry_state": "平静",
                "exit_state": "紧张",
                "dominant_panel_number": 1,
                "panels": [{
                    "panel_number": 1,
                    "beat_refs": [{
                        "scene_index": 1,
                        "shot_index": 1,
                        "beat_index": 1
                    }],
                    "function": "dialogue",
                    "size": "dominant",
                    "shot_scale": "medium",
                    "camera_angle": "eye_level",
                    "participants": ["主角"],
                    "composition": "单人构图",
                    "observable_action": "主角抬头",
                    "reaction": null,
                    "spatial_relationship": "画面中央",
                    "continuity_from_previous": null,
                    "text_layers": []
                }]
            },
            "drawing_prompt": {
                "schema_version": "manga_page_drawing_prompt_v2",
                "page_number": 1,
                "image_prompt": "整页提示词"
            },
            "reference_bindings": [{
                "character_name": "主角",
                "bound_asset": null
            }],
            "prompt_fingerprint": {"template": "sha256"},
            "source": {
                "root_project_id": 33,
                "source_chapter_id": 22,
                "source_snapshot_id": null,
                "source_snapshot_chapter_id": null,
                "source_revision_key": "chapter:22:revision:1",
                "content_sha256": "content-sha256",
                "shooting_script_id": 71,
                "shooting_script_revision": 2,
                "shooting_script_sha256": "script-sha256"
            }
        })
    }

    #[test]
    fn projects_canonical_layout_without_db_only_fields() {
        let summary = manga_page_prompt_summary(&page_prompt(Some(valid_layout())))
            .expect("valid layout should project");

        assert_eq!(summary.id, 11);
        assert_eq!(summary.page_number, 1);
        assert_eq!(summary.source.shooting_script_id, 71);
        assert_eq!(summary.reference_bindings[0].character_name, "主角");
        let output = serde_json::to_value(summary).expect("summary should serialize");
        assert!(output.get("shooting_script_id").is_none());
        assert!(output.get("shooting_script_revision").is_none());
        assert!(output.get("prompt_type").is_none());
        assert!(output.get("layout").is_none());
        assert!(output.get("characters_in_frame").is_none());
    }

    #[test]
    fn rejects_missing_unknown_and_invalid_layouts() {
        assert_eq!(
            manga_page_prompt_summary(&page_prompt(None)).unwrap_err(),
            "漫画页面提示词 #11 缺少 v2 layout"
        );

        let mut unknown = valid_layout();
        unknown["legacy_notes"] = json!("旧字段");
        let error = manga_page_prompt_summary(&page_prompt(Some(unknown))).unwrap_err();
        assert!(error.contains("漫画页面提示词 #11 的 v2 layout 无效"));
        assert!(error.contains("unknown field"));

        let mut invalid = valid_layout();
        invalid["source"]["shooting_script_revision"] = json!(0);
        let error = manga_page_prompt_summary(&page_prompt(Some(invalid))).unwrap_err();
        assert!(error.contains("source.shooting_script_revision"));
    }
}
