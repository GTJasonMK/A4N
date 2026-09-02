use a4n_core::error::AppError;
use a4n_prompt::{chapter_analysis::ChapterAnalysisRequestStep, rendering::PromptRenderer};
use serde_json::json;
use std::sync::Arc;

use crate::OrchestratorContext;

use super::{
    context::{
        build_scene_profile_sync_request_with_profiles, load_scene_profile_sync_anchor_context,
        load_scene_profile_sync_stable_context, select_scene_profile_sync_chapters,
    },
    stages::build_scene_profile_stage_request,
    types::{
        ChapterAnchorMode, SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE, SCENE_PROFILE_CANDIDATES_PROMPT,
        SCENE_TYPE_RULE,
    },
};

impl OrchestratorContext {
    pub(crate) async fn prepare_scene_profile_sync_preview_validated(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
        let chapters = select_scene_profile_sync_chapters(self, project_id, chapter_number)
            .await
            .map_err(scene_profile_sync_preview_error)?;
        let (stable, existing_scene_profiles, anchors) = futures::join!(
            load_scene_profile_sync_stable_context(self, project_id),
            async {
                self.scene_profiles_by_project(project_id)
                    .await
                    .map_err(|error| error.to_string())
            },
            load_scene_profile_sync_anchor_context(
                self,
                project_id,
                &chapters,
                ChapterAnchorMode::RequireExistingCurrent,
            ),
        );
        let stable = stable.map_err(scene_profile_sync_preview_error)?;
        let existing_scene_profiles =
            Arc::new(existing_scene_profiles.map_err(scene_profile_sync_preview_error)?);
        let anchors = anchors.map_err(scene_profile_sync_preview_error)?;
        let mut steps = Vec::new();
        for chapter in chapters {
            let chapter_id = chapter.id;
            let context = build_scene_profile_sync_request_with_profiles(
                chapter,
                anchors
                    .get(&chapter_id)
                    .cloned()
                    .ok_or_else(|| "场景档案同步锚点批量读取缺少章节".to_string())
                    .map_err(scene_profile_sync_preview_error)?,
                &stable,
                Arc::clone(&existing_scene_profiles),
            );
            let chapter_number = context.chapter.chapter_number;
            let scene_request = build_scene_profile_stage_request(
                prompt_renderer,
                &context,
                None,
                SCENE_PROFILE_CANDIDATES_PROMPT,
                "识别或匹配当前章节中的可复用场景档案，只输出 scenes",
                json!({}),
                json!({
                    "format": "json_object",
                    "required_root_field": "scenes",
                    "strict_rules": [
                        "只返回 scenes 数组，不要输出 occurrences、transitions 或 entity_states",
                        SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE,
                        SCENE_TYPE_RULE,
                        "importance 只能是 primary/secondary/minor",
                        "evidence_anchor_ids 必须来自 input.text_anchors[].id",
                        "scene_profile_id 和 parent_scene_id 只能引用 input.existing_scene_profiles[].id 或 null"
                    ]
                }),
            )
            .await
            .map_err(scene_profile_sync_preview_error)?;
            steps.push(ChapterAnalysisRequestStep {
                key: format!("scene_profile_sync_ch{chapter_number}_scenes"),
                title: format!("第 {chapter_number} 章场景档案候选识别请求"),
                prompt_name: SCENE_PROFILE_CANDIDATES_PROMPT.to_string(),
                request: scene_request,
            });
        }
        Ok(steps)
    }
}

fn scene_profile_sync_preview_error(message: String) -> AppError {
    AppError::InvalidState {
        current: "scene_profile_sync_preview".to_string(),
        operation: message,
    }
}
