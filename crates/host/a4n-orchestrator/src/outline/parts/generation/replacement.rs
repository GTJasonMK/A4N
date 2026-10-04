use std::collections::HashMap;

use a4n_domain::chapter::PartOutlineRecord;
use a4n_error::AppError;
use chrono::Utc;

use crate::outline::context::{
    load_part_generation_context, with_locked_outline_generation_sources,
};
use crate::outline::part_contract::{
    PartOutlineDraft, part_outline_model_from_json, sync_part_outlines_and_assign_chapters_in,
    validate_existing_parts_for_plan,
};
use crate::outline::{OrchestratorContext, PartOutlineRegenerationInput, report_progress};

use super::super::contracts::PartOutlineJsonInput;
use super::super::planning::{plan_replacement_parts, validate_replacement_part_sources};

impl OrchestratorContext {
    pub(in crate::outline) async fn regenerate_parts(
        &self,
        input: PartOutlineRegenerationInput<'_, '_>,
    ) -> Result<Vec<PartOutlineRecord>, AppError> {
        let PartOutlineRegenerationInput {
            project_id,
            expected_parts,
            chapters_per_part,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let mut context = load_part_generation_context(
            self.database.executor(),
            project_id,
            None,
            chapters_per_part,
        )
        .await?;
        validate_existing_parts_for_plan(&context.parts)?;
        context.requested_part_count =
            validate_replacement_part_sources(&context.parts, expected_parts)?;
        let batch =
            plan_replacement_parts(context.requested_part_count, context.chapters_per_part)?;
        let num_parts = i32::try_from(batch.len()).map_err(|_| {
            AppError::InvalidArgument(format!("待重生成分部数量超过 i32 范围：{}", batch.len()))
        })?;
        let now = Utc::now().naive_utc();
        let mut generated_jsons = Vec::new();
        let mut generated_context_parts = Vec::new();

        report_progress(
            progress,
            0,
            num_parts,
            &format!("准备重新生成 {num_parts} 个分部大纲"),
        )?;
        for (index, (part_num, start_ch, end_ch)) in batch.into_iter().enumerate() {
            let progress_index = i32::try_from(index).map_err(|_| {
                AppError::InvalidArgument(format!("分部重生成进度索引超过 i32 范围：{index}"))
            })?;
            report_progress(
                progress,
                progress_index,
                num_parts,
                &format!("正在重新生成第 {part_num}/{num_parts} 部"),
            )?;
            let generated = self
                .generate_part_outline_json(PartOutlineJsonInput {
                    project_id,
                    part_num,
                    start_ch,
                    end_ch,
                    chapters_per_part: context.chapters_per_part,
                    requested_part_count: context.requested_part_count,
                    blueprint_context: &context.blueprint_payload,
                    previous_parts: &generated_context_parts,
                    source_snapshot: &context.source_snapshot,
                    prompt_renderer,
                    variables,
                    workflow_run_id,
                })
                .await?;
            generated_context_parts.push(part_outline_model_from_json(
                0,
                project_id,
                part_num,
                start_ch,
                end_ch,
                &generated.data,
                now,
            )?);
            generated_jsons.push((part_num, start_ch, end_ch, generated));
            report_progress(
                progress,
                progress_index + 1,
                num_parts,
                &format!("已重新生成第 {part_num}/{num_parts} 部"),
            )?;
        }

        report_progress(progress, num_parts, num_parts, "正在增量保存分部并重绑章纲")?;
        let drafts = generated_jsons
            .iter()
            .map(
                |(part_number, start_chapter, end_chapter, generated)| PartOutlineDraft {
                    part_number: *part_number,
                    start_chapter: *start_chapter,
                    end_chapter: *end_chapter,
                    data: generated.data.clone(),
                },
            )
            .collect();
        let results = with_locked_outline_generation_sources(
            self.database.executor(),
            project_id,
            &context.source_snapshot,
            |tx| {
                Box::pin(async move {
                    let result = async {
                        let results =
                            sync_part_outlines_and_assign_chapters_in(&tx, project_id, drafts, now)
                                .await?;
                        let mut generated_by_part_number =
                            HashMap::with_capacity(generated_jsons.len());
                        for (part_number, _, _, generated) in &generated_jsons {
                            generated_by_part_number
                                .entry(*part_number)
                                .or_insert(generated);
                        }
                        for part in &results {
                            if let Some(generated) = generated_by_part_number.get(&part.part_number)
                            {
                                self.record_part_outline_generation_run(
                                    &tx,
                                    part,
                                    generated,
                                    "重新生成分部大纲",
                                )
                                .await?;
                            }
                        }
                        Ok(results)
                    }
                    .await;
                    (tx, result)
                })
                    as a4n_db::transaction::TransactionWork<'_, Vec<PartOutlineRecord>>
            },
        )
        .await?;
        Ok(results)
    }
}
