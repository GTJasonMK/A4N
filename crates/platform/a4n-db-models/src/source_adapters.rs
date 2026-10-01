//! Adapters from persistence rows to pure domain input contracts.

use a4n_domain::{
    chapter::ChapterAnalysisSource,
    source_spans::{SourceChapter, SourceTextAnchor},
    video_prompt::ChapterVideoPromptSummary,
};

use crate::models::{
    chapter::{chapter, chapter_video_prompt},
    project::text_anchor,
};

impl SourceChapter for chapter::Model {
    fn source_chapter_id(&self) -> i64 {
        self.id
    }

    fn source_chapter_number(&self) -> i32 {
        self.chapter_number
    }

    fn source_content(&self) -> &str {
        &self.content
    }
}

impl SourceTextAnchor for text_anchor::Model {
    fn source_anchor_id(&self) -> i64 {
        self.id
    }

    fn source_anchor_chapter_id(&self) -> Option<i64> {
        self.source_chapter_id
    }

    fn source_anchor_quote(&self) -> &str {
        &self.quote
    }

    fn source_anchor_char_range(&self) -> (i32, i32) {
        (self.start_offset, self.end_offset)
    }
}

impl ChapterAnalysisSource for chapter::Model {
    fn has_available_analysis_content(&self) -> bool {
        self.has_available_content()
    }
}

impl From<&chapter_video_prompt::Model> for ChapterVideoPromptSummary {
    fn from(prompt: &chapter_video_prompt::Model) -> Self {
        Self {
            id: prompt.id,
            project_id: prompt.project_id,
            chapter_id: prompt.chapter_id,
            style: prompt.style.clone(),
            prompt_type: prompt.prompt_type.clone(),
            workflow_mode: prompt.workflow_mode.clone(),
            shot_number: prompt.shot_number,
            title: prompt.title.clone(),
            scene_summary: prompt.scene_summary.clone(),
            visual_prompt: prompt.visual_prompt.clone(),
            motion_prompt: prompt.motion_prompt.clone(),
            negative_prompt: prompt.negative_prompt.clone(),
            duration_secs: prompt.duration_secs,
            aspect_ratio: prompt.aspect_ratio.clone(),
            reference_image_ids_json: prompt.reference_image_ids_json.clone(),
            status: prompt.status.clone(),
            generated_video_id: prompt.generated_video_id,
            metadata_json: prompt.metadata_json.clone(),
            created_at: prompt.created_at.to_string(),
            updated_at: prompt.updated_at.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::*;

    #[test]
    fn chapter_video_prompt_summary_projects_record_fields() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 24)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let updated_at = created_at + chrono::Duration::minutes(1);
        let prompt = chapter_video_prompt::Model {
            id: 9,
            project_id: 4,
            chapter_id: 5,
            style: Some("cinematic".to_string()),
            prompt_type: "video_prompt".to_string(),
            workflow_mode: "storyboard".to_string(),
            shot_number: 3,
            title: Some("雨夜相遇".to_string()),
            scene_summary: Some("车站灯光下相遇".to_string()),
            visual_prompt: "wide shot".to_string(),
            motion_prompt: Some("slow push in".to_string()),
            negative_prompt: Some("text".to_string()),
            duration_secs: 8,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: Some(json!([12, 13])),
            status: "completed".to_string(),
            generated_video_id: Some(20),
            timeline_item_id: Some(21),
            metadata_json: Some(json!({"seed": 17})),
            created_at,
            updated_at,
        };

        let summary = ChapterVideoPromptSummary::from(&prompt);

        assert_eq!(summary.id, prompt.id);
        assert_eq!(summary.project_id, prompt.project_id);
        assert_eq!(summary.chapter_id, prompt.chapter_id);
        assert_eq!(summary.prompt_type, prompt.prompt_type);
        assert_eq!(summary.workflow_mode, prompt.workflow_mode);
        assert_eq!(
            summary.reference_image_ids_json,
            prompt.reference_image_ids_json
        );
        assert_eq!(summary.metadata_json, prompt.metadata_json);
        assert_eq!(summary.created_at, created_at.to_string());
        assert_eq!(summary.updated_at, updated_at.to_string());
    }
}
