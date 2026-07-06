use a4n_core::error::AppError;
use a4n_core::models::{
    character::protagonist_profile,
    project::{source_snapshot_chapter, text_anchor},
};
use a4n_engine::prompt::request::build_json_task_user_message;
use serde::Serialize;
use serde_json::{Value, json};

use super::{GalgamePipelineSourceSpan, GalgameSpeakerProfile};

const JSON_RESPONSE_REQUIREMENT: &str =
    "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelinePromptChapter {
    pub source_snapshot_chapter_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content_hash: String,
    pub content: String,
    pub source_spans: Vec<GalgamePipelinePromptSourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelinePromptSourceSpan {
    pub id: String,
    pub text_anchor_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub text: String,
}

pub fn galgame_prompt_chapter_from_snapshot(
    chapter: &source_snapshot_chapter::Model,
    anchors: &[text_anchor::Model],
) -> Result<GalgamePipelinePromptChapter, AppError> {
    let mut source_spans = anchors
        .iter()
        .filter(|anchor| anchor.source_snapshot_chapter_id == Some(chapter.id))
        .map(|anchor| {
            let text = anchor.quote.trim();
            if text.is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame prompt text_anchor {} quote 不能为空",
                    anchor.id
                )));
            }
            Ok(GalgamePipelinePromptSourceSpan {
                id: galgame_source_span_id(chapter.chapter_number, anchor.id),
                text_anchor_id: anchor.id,
                source_snapshot_chapter_id: chapter.id,
                paragraph_index: anchor.paragraph_index,
                start_offset: anchor.start_offset,
                end_offset: anchor.end_offset,
                text: text.to_string(),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    source_spans.sort_by_key(|span| (span.paragraph_index, span.start_offset, span.text_anchor_id));
    if source_spans.is_empty() {
        return Err(AppError::InvalidState {
            current: format!("source_snapshot_chapter_{}_has_no_prompt_spans", chapter.id),
            operation: "build_galgame_pipeline_prompt_chapter".to_string(),
        });
    }
    Ok(GalgamePipelinePromptChapter {
        source_snapshot_chapter_id: chapter.id,
        source_chapter_id: chapter.source_chapter_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        summary: chapter.summary.clone(),
        content_hash: chapter.content_hash.clone(),
        content: chapter.content.clone(),
        source_spans,
    })
}

pub fn galgame_pipeline_source_spans_from_prompt_chapters(
    chapters: &[GalgamePipelinePromptChapter],
) -> Vec<GalgamePipelineSourceSpan> {
    chapters
        .iter()
        .flat_map(|chapter| {
            chapter
                .source_spans
                .iter()
                .map(|span| GalgamePipelineSourceSpan {
                    id: span.id.clone(),
                    text_anchor_id: span.text_anchor_id,
                    source_snapshot_chapter_id: span.source_snapshot_chapter_id,
                })
        })
        .collect()
}

pub fn galgame_speaker_profiles_from_models(
    profiles: &[protagonist_profile::Model],
) -> Vec<GalgameSpeakerProfile> {
    profiles
        .iter()
        .map(|profile| GalgameSpeakerProfile {
            id: profile.id,
            name: profile.name.clone(),
        })
        .collect()
}

pub fn galgame_scene_segmentation_user_message(
    chapters: &[GalgamePipelinePromptChapter],
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_scene_segmentation",
        "将源快照章节切分为 Galgame 场景清单",
        json!({
            "chapters": chapters,
        }),
        json!({
            "format": "json_object",
            "required_fields": ["scenes"],
            "source_span_rule": "scenes[].source_span_ids 必须从 input.chapters[].source_spans[].id 中选择，至少 1 个，不要自造 ID",
            "index_rule": "scenes[].index 从 1 开始连续递增"
        }),
    )
}

pub fn galgame_script_adaptation_user_message(
    chapters: &[GalgamePipelinePromptChapter],
    scene_segmentation: &Value,
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_script_adaptation",
        "把已切分场景改编为 Galgame 剧本行草稿",
        json!({
            "chapters": chapters,
            "previous_step": {
                "scene_segmentation": scene_segmentation,
            }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["scene_lines"],
            "line_types": ["dialogue", "narration", "monologue", "stage_direction"],
            "source_span_rule": "每个 lines[].source_span_ids 必须从 input.chapters[].source_spans[].id 中选择，至少 1 个，不要输出空数组或自造 ID",
            "index_rule": "scene_lines[].lines[].index 从 1 开始连续递增"
        }),
    )
}

pub fn galgame_character_staging_user_message(
    script_adaptation: &Value,
    speaker_profiles: &[GalgameSpeakerProfile],
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_character_staging",
        "绑定剧本行说话人到人物档案，并标注表情、姿势和站位",
        json!({
            "previous_step": {
                "script_adaptation": script_adaptation,
            },
            "speaker_profiles": speaker_profiles,
        }),
        json!({
            "format": "json_object",
            "required_fields": ["scene_lines"],
            "profile_rule": "speaker_profile_id 只能使用 input.speaker_profiles[].id；无法可靠绑定时为 null",
            "expression_values": ["neutral", "smile", "sad", "angry", "surprised", "shy", "serious"],
            "position_values": ["left", "center", "right"],
            "source_span_rule": "必须原样保留每行非空 source_span_ids，不要输出空数组或自造 ID"
        }),
    )
}

pub fn galgame_visual_planning_user_message(
    scene_segmentation: &Value,
    character_staging: &Value,
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_visual_planning",
        "为 Galgame 场景规划背景需求，并挑选事件 CG 候选行",
        json!({
            "previous_steps": {
                "scene_segmentation": scene_segmentation,
                "character_staging": character_staging,
            }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["backgrounds", "event_cgs"],
            "background_rule": "每个 scene_index 必须有一个 backgrounds 条目",
            "event_cg_rule": "event_cgs[].line_index 必须引用输入剧本行；普通对白不要生成 CG",
            "importance_values": ["normal", "high", "critical"]
        }),
    )
}

pub fn galgame_branch_design_user_message(
    scene_segmentation: &Value,
    character_staging: &Value,
) -> Result<String, AppError> {
    build_galgame_step_user_message(
        "galgame_branch_design",
        "为 Galgame 设计珍珠串结构选择支和回收点",
        json!({
            "previous_steps": {
                "scene_segmentation": scene_segmentation,
                "character_staging": character_staging,
            }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["choices"],
            "choice_rule": "每个选择点 2 到 3 个选项；每个选项会物化为一个短支线 branch 场，再通过 options[].returns_to_scene_index 回到已有主线场；returns_to_scene_index 必须填写并引用 scene_segmentation.scenes[].index",
            "phase": "可返回空 choices 数组；非空 choices 会持久化为 choice 行、branch 场与 galgame_choice 选项"
        }),
    )
}

fn build_galgame_step_user_message(
    prompt_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<String, AppError> {
    build_json_task_user_message(
        "执行 Galgame 管线步骤",
        JSON_RESPONSE_REQUIREMENT,
        &json!({
            "task": {
                "prompt_name": prompt_name,
                "instruction": instruction,
            },
            "output_contract": output_contract,
            "input": input,
        }),
    )
}

fn galgame_source_span_id(chapter_number: i32, text_anchor_id: i64) -> String {
    format!("ch{chapter_number}:ta{text_anchor_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::Value;

    #[test]
    fn builds_prompt_chapter_from_snapshot_anchors() {
        let chapter = snapshot_chapter(20, 1, "雨夜车站。她回过头。");
        let anchors = vec![
            text_anchor(101, 20, 2, 6, 11, "她回过头"),
            text_anchor(100, 20, 1, 0, 5, "雨夜车站"),
        ];

        let prompt_chapter =
            galgame_prompt_chapter_from_snapshot(&chapter, &anchors).expect("prompt chapter");

        assert_eq!(prompt_chapter.source_spans.len(), 2);
        assert_eq!(prompt_chapter.source_spans[0].id, "ch1:ta100");
        assert_eq!(prompt_chapter.source_spans[1].text_anchor_id, 101);
    }

    #[test]
    fn prompt_chapter_rejects_missing_snapshot_anchors_without_content_fallback() {
        let chapter = snapshot_chapter(20, 1, "雨夜车站。她回过头。");
        let anchors = vec![text_anchor(100, 21, 1, 0, 5, "其他章节")];

        let err = galgame_prompt_chapter_from_snapshot(&chapter, &anchors)
            .expect_err("Galgame prompt chapter must require same-chapter text anchors");

        assert!(
            err.to_string()
                .contains("source_snapshot_chapter_20_has_no_prompt_spans")
        );
        assert!(
            err.to_string()
                .contains("build_galgame_pipeline_prompt_chapter")
        );
    }

    #[test]
    fn builds_persist_source_spans_from_prompt_chapters() {
        let chapter = galgame_prompt_chapter_from_snapshot(
            &snapshot_chapter(20, 1, "雨夜车站。"),
            &[text_anchor(100, 20, 1, 0, 5, "雨夜车站")],
        )
        .expect("prompt chapter");

        let spans = galgame_pipeline_source_spans_from_prompt_chapters(&[chapter]);

        assert_eq!(spans[0].id, "ch1:ta100");
        assert_eq!(spans[0].text_anchor_id, 100);
        assert_eq!(spans[0].source_snapshot_chapter_id, 20);
    }

    #[test]
    fn scene_segmentation_message_contains_source_span_contract() {
        let chapter = galgame_prompt_chapter_from_snapshot(
            &snapshot_chapter(20, 1, "雨夜车站。"),
            &[text_anchor(100, 20, 1, 0, 5, "雨夜车站")],
        )
        .expect("prompt chapter");

        let message = galgame_scene_segmentation_user_message(&[chapter]).expect("message");
        let payload = message
            .split_once("\n\n输入：\n")
            .map(|(_, payload)| payload)
            .expect("payload marker");
        let value: Value = serde_json::from_str(payload).expect("payload json");

        assert_eq!(
            value["task"]["prompt_name"],
            json!("galgame_scene_segmentation")
        );
        assert_eq!(
            value["input"]["chapters"][0]["source_spans"][0]["id"],
            json!("ch1:ta100")
        );
        assert!(
            value["output_contract"]["source_span_rule"]
                .as_str()
                .unwrap()
                .contains("source_spans")
        );
    }

    #[test]
    fn character_staging_message_includes_speaker_profiles() {
        let message = galgame_character_staging_user_message(
            &json!({"scene_lines": []}),
            &[GalgameSpeakerProfile {
                id: 7,
                name: "林晓雨".to_string(),
            }],
        )
        .expect("message");

        assert!(message.contains("\"speaker_profiles\""));
        assert!(message.contains("\"林晓雨\""));
    }

    fn snapshot_chapter(
        id: i64,
        chapter_number: i32,
        content: &str,
    ) -> source_snapshot_chapter::Model {
        source_snapshot_chapter::Model {
            id,
            snapshot_id: 10,
            root_project_id: 1,
            source_chapter_id: 5,
            chapter_number,
            title: Some("测试章节".to_string()),
            content: content.to_string(),
            summary: Some("章节摘要".to_string()),
            analysis_data_json: None,
            content_hash: "hash".to_string(),
            analysis_hash: None,
        }
    }

    fn text_anchor(
        id: i64,
        source_snapshot_chapter_id: i64,
        paragraph_index: i32,
        start_offset: i32,
        end_offset: i32,
        quote: &str,
    ) -> text_anchor::Model {
        text_anchor::Model {
            id,
            project_id: 1,
            source_chapter_id: Some(5),
            source_snapshot_id: Some(10),
            source_snapshot_chapter_id: Some(source_snapshot_chapter_id),
            source_kind: "chapter".to_string(),
            source_id: 5,
            paragraph_index,
            start_offset,
            end_offset,
            quote: quote.to_string(),
            quote_hash: "quote_hash".to_string(),
            source_version_hash: "source_hash".to_string(),
            created_at: Utc::now().naive_utc(),
        }
    }
}
