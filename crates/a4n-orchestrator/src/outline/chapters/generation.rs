use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use chrono::Utc;

use crate::outline::chapter_contract::{
    ensure_focus_part_in_context, insert_chapter_outline_from_json, requested_part_for_chapter,
    required_part_summary, validate_chapter_generation_range,
    validate_chapter_outline_generation_request, validate_requested_part_context,
};
use crate::outline::context::{
    load_chapter_outline_generation_context, lock_outline_generation_sources,
};
use crate::outline::{
    ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput, OrchestratorContext,
    report_progress, validate_existing_chapter_outline_for_request,
};

use super::contracts::{ChapterOutlineSequenceInput, SingleChapterOutlineJsonInput};
use super::queries::existing_chapter_outline_for_number;

impl OrchestratorContext {
    /// 生成章节大纲（增量）
    pub(in crate::outline) async fn generate_chapter_outlines(
        &self,
        input: ChapterOutlineGenerationInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let ChapterOutlineGenerationInput {
            project_id,
            part_id,
            start_chapter,
            count,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let part = self.get_part(part_id).await?;
        if part.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        let (start_num, end_num) = validate_chapter_generation_range(&part, start_chapter, count)?;
        self.generate_chapter_outline_sequence(ChapterOutlineSequenceInput {
            project_id,
            start_chapter: start_num,
            count: end_num - start_num + 1,
            focus_part: Some(&part),
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        })
        .await
    }

    /// 按数量追加章节大纲（短篇项目或已有分部范围内的追加生成）。
    pub(in crate::outline) async fn generate_chapter_outlines_by_count(
        &self,
        input: ChapterOutlineCountGenerationInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let ChapterOutlineCountGenerationInput {
            project_id,
            start_chapter,
            count,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        } = input;
        self.generate_chapter_outline_sequence(ChapterOutlineSequenceInput {
            project_id,
            start_chapter,
            count,
            focus_part: None,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        })
        .await
    }

    async fn generate_chapter_outline_sequence(
        &self,
        input: ChapterOutlineSequenceInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let ChapterOutlineSequenceInput {
            project_id,
            start_chapter,
            count,
            focus_part,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let end_chapter = validate_chapter_outline_generation_request(start_chapter, count)?;
        if let Some(part) = focus_part {
            required_part_summary(part)?;
            if start_chapter < part.start_chapter || end_chapter > part.end_chapter {
                return Err(AppError::InvalidArgument(format!(
                    "章节大纲生成范围 {start_chapter}..={end_chapter} 超出分部范围 {}..={}",
                    part.start_chapter, part.end_chapter
                )));
            }
        }

        let mut context =
            load_chapter_outline_generation_context(self.database.connection(), project_id).await?;
        validate_requested_part_context(&context.parts, start_chapter, end_chapter)?;
        ensure_focus_part_in_context(focus_part, &context.parts)?;

        report_progress(progress, 0, count, &format!("准备生成 {count} 个章节大纲"))?;
        let mut results = Vec::new();
        for chapter_number in start_chapter..=end_chapter {
            let current = chapter_number - start_chapter + 1;
            let current_focus_part = focus_part
                .map(|focus_part| {
                    context
                        .parts
                        .iter()
                        .find(|part| part.id == focus_part.id)
                        .ok_or_else(|| AppError::InvalidState {
                            current: "outline_generation_source_changed".to_string(),
                            operation: "生成章节大纲（目标分部已变化，请重新生成）".to_string(),
                        })
                })
                .transpose()?;
            let expected_part_id =
                requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);
            if current_focus_part.is_some_and(|part| {
                chapter_number < part.start_chapter
                    || chapter_number > part.end_chapter
                    || expected_part_id != Some(part.id)
            }) {
                return Err(AppError::InvalidState {
                    current: "outline_generation_source_changed".to_string(),
                    operation: "生成章节大纲（目标分部范围已变化，请重新生成）".to_string(),
                });
            }

            if let Some(existing) =
                existing_chapter_outline_for_number(&context.chapter_outlines, chapter_number)?
            {
                validate_existing_chapter_outline_for_request(
                    &existing,
                    chapter_number,
                    expected_part_id,
                )?;
                // 该分支没有写入；生成分支会在提交后刷新 context，这里复用现有快照即可。
                report_progress(
                    progress,
                    current,
                    count,
                    &format!("第 {current}/{count} 个章节大纲已存在，保留第 {chapter_number} 章"),
                )?;
                results.push(existing);
                continue;
            }

            report_progress(
                progress,
                current - 1,
                count,
                &format!("正在生成第 {current}/{count} 个章节大纲（第 {chapter_number} 章）"),
            )?;
            let generated = self
                .generate_single_chapter_outline_json(SingleChapterOutlineJsonInput {
                    project_id,
                    context: &context,
                    focus_part: current_focus_part,
                    chapter_number,
                    prompt_renderer,
                    variables,
                    workflow_run_id,
                })
                .await?;
            let part_id =
                requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);
            let tx = lock_outline_generation_sources(
                self.database.connection(),
                project_id,
                &context.source_snapshot,
            )
            .await?;
            let outline = insert_chapter_outline_from_json(
                &tx,
                project_id,
                part_id,
                &generated.chapter_json,
                Utc::now().naive_utc(),
            )
            .await?;
            tx.commit().await.map_err(AppError::Database)?;
            self.record_chapter_outline_generation_run(
                &outline,
                &generated.request,
                &generated.response_content,
                generated.finish_reason.as_deref(),
                "生成章节大纲",
            )
            .await?;
            results.push(outline);
            context =
                load_chapter_outline_generation_context(self.database.connection(), project_id)
                    .await?;
            report_progress(
                progress,
                current,
                count,
                &format!("已生成第 {current}/{count} 个章节大纲（第 {chapter_number} 章）"),
            )?;
        }

        Ok(results)
    }
}
