use a4n_domain::chapter::PartOutlineRecord;
use a4n_error::AppError;
use chrono::Utc;

use crate::outline::context::{load_part_generation_context, lock_outline_generation_sources};
use crate::outline::part_contract::{
    insert_part_outline_from_json, validate_existing_parts_for_plan,
};
use crate::outline::{OrchestratorContext, PartOutlineGenerationInput, report_progress};

use super::super::contracts::PartOutlineJsonInput;
use super::super::planning::plan_next_parts;

impl OrchestratorContext {
    pub(in crate::outline) async fn generate_parts(
        &self,
        input: PartOutlineGenerationInput<'_, '_>,
    ) -> Result<Vec<PartOutlineRecord>, AppError> {
        let PartOutlineGenerationInput {
            project_id,
            part_count,
            chapters_per_part,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let mut context =
            load_part_generation_context(&self.database, project_id, part_count, chapters_per_part)
                .await?;
        let batch = plan_next_parts(
            &context.parts,
            context.requested_part_count,
            context.chapters_per_part,
        )?;
        let num_parts = i32::try_from(batch.len()).map_err(|_| {
            AppError::InvalidArgument(format!("待生成分部数量超过 i32 范围：{}", batch.len()))
        })?;
        validate_existing_parts_for_plan(&context.parts)?;
        let mut results = context.parts.clone();
        report_progress(
            progress,
            0,
            num_parts,
            &format!("准备生成 {num_parts} 个分部大纲"),
        )?;

        for (index, (part_num, start_ch, end_ch)) in batch.into_iter().enumerate() {
            let progress_index = i32::try_from(index).map_err(|_| {
                AppError::InvalidArgument(format!("分部生成进度索引超过 i32 范围：{index}"))
            })?;
            report_progress(
                progress,
                progress_index,
                num_parts,
                &format!("正在生成第 {part_num}/{num_parts} 部"),
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
                    previous_parts: &results,
                    source_snapshot: &context.source_snapshot,
                    prompt_renderer,
                    variables,
                    workflow_run_id,
                })
                .await?;
            let tx = lock_outline_generation_sources(
                &self.database,
                project_id,
                &context.source_snapshot,
            )
            .await?;
            let part = insert_part_outline_from_json(
                &tx,
                project_id,
                part_num,
                start_ch,
                end_ch,
                &generated.data,
                Utc::now().naive_utc(),
            )
            .await?;
            self.record_part_outline_generation_run(&tx, &part, &generated, "生成分部大纲")
                .await?;
            a4n_db::commit_transaction(tx).await?;
            context = load_part_generation_context(
                &self.database,
                project_id,
                Some(context.requested_part_count),
                Some(context.chapters_per_part),
            )
            .await?;
            results = context.parts.clone();
            report_progress(
                progress,
                progress_index + 1,
                num_parts,
                &format!("已生成第 {part_num}/{num_parts} 部"),
            )?;
        }

        Ok(results)
    }
}
