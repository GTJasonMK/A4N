use a4n_core::error::AppError;
use a4n_db::galgame::GalgamePipelinePromptContextRecord;

use crate::galgame::{
    GalgameChapterShootingScript, GalgamePipelinePromptContext, OrchestratorContext,
    galgame_pipeline_source_spans_from_prompt_chapters, galgame_prompt_chapter_from_snapshot,
    galgame_speaker_profiles_from_models, validate_positive_id,
};
use crate::shooting_script::ChapterShootingScriptSourceInput;

impl OrchestratorContext {
    pub async fn galgame_pipeline_prompt_context(
        &self,
        project_id: i64,
    ) -> Result<GalgamePipelinePromptContext, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let record = a4n_db::galgame::galgame_pipeline_prompt_context_record(
            self.database.connection(),
            context.root_project.id,
            context.source_snapshot.id,
        )
        .await
        .map_err(AppError::Database)?;
        let mut context = galgame_pipeline_prompt_context_from_record(
            context.project.id,
            context.root_project.id,
            context.source_snapshot.id,
            record,
        )?;
        let mut shooting_scripts = Vec::with_capacity(context.chapters.len());
        for chapter in &context.chapters {
            let resolved = self
                .require_current_chapter_shooting_script(ChapterShootingScriptSourceInput {
                    project_id,
                    source_chapter_id: chapter.source_chapter_id,
                })
                .await?;
            if resolved.source.source_snapshot_chapter_id
                != Some(chapter.source_snapshot_chapter_id)
                || resolved.source.chapter_number != chapter.chapter_number
            {
                return Err(AppError::InvalidState {
                    current: format!(
                        "galgame_shooting_script_source_mismatch:source_chapter_id={}",
                        chapter.source_chapter_id
                    ),
                    operation: "装配 Galgame 结构化拍摄剧本".to_string(),
                });
            }
            shooting_scripts.push(GalgameChapterShootingScript {
                source_chapter_id: chapter.source_chapter_id,
                source_snapshot_chapter_id: chapter.source_snapshot_chapter_id,
                chapter_number: chapter.chapter_number,
                shooting_script_id: resolved.shooting_script.record.id,
                shooting_script_revision: resolved.shooting_script.record.revision,
                script: resolved.shooting_script.script,
            });
        }
        context.shooting_scripts = shooting_scripts;
        Ok(context)
    }
}

/// 将 DB 层管线提示词上下文 read model 装配为 service 侧 DTO。
pub(crate) fn galgame_pipeline_prompt_context_from_record(
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
    record: GalgamePipelinePromptContextRecord,
) -> Result<GalgamePipelinePromptContext, AppError> {
    if record.chapters.is_empty() {
        return Err(AppError::InvalidState {
            current: "source_snapshot_has_no_chapters".to_string(),
            operation: "load_galgame_pipeline_prompt_context".to_string(),
        });
    }
    let mut prompt_chapters = Vec::with_capacity(record.chapters.len());
    for chapter in record.chapters {
        prompt_chapters.push(galgame_prompt_chapter_from_snapshot(
            &chapter.chapter,
            &chapter.text_anchors,
        )?);
    }
    let source_spans = galgame_pipeline_source_spans_from_prompt_chapters(&prompt_chapters);
    Ok(GalgamePipelinePromptContext {
        project_id,
        root_project_id,
        source_snapshot_id,
        speaker_profiles: galgame_speaker_profiles_from_models(&record.speaker_profiles),
        chapters: prompt_chapters,
        source_spans,
        shooting_scripts: Vec::new(),
    })
}
