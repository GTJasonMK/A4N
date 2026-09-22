use super::mutability::{
    PromptDependencyIndex, ensure_no_generated_frame_image,
    ensure_prompt_dependencies_mutable_with_index, ensure_prompt_stage_mutability_preconditions,
};
use super::{frame_images::generated_frame_images_for_prompts, rules::prompt_stage};
use crate::video::prompts::consistency::PromptStage;
use crate::video::{self, VideoDbError};
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::image::image_generation_prompt_id;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};
use std::collections::{BTreeSet, HashMap};

pub async fn delete_chapter_video_prompts_with_policy<C>(
    db: &C,
    prompt_ids: &[i64],
    allow_downstream: bool,
) -> Result<u64, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let prompt_ids_set = prompt_ids.iter().copied().collect::<BTreeSet<_>>();
    if prompt_ids_set.is_empty() {
        return Ok(0);
    }
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids_set.iter().copied()))
        .all(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    if rows.len() != prompt_ids_set.len() {
        return Err(VideoDbError::NotFound(if prompt_ids_set.len() == 1 {
            "视频提示词不存在".to_string()
        } else {
            "部分视频提示词不存在，不能批量删除".to_string()
        }));
    }
    let mut sibling_rows_by_scope = HashMap::new();
    let mut dependency_indexes_by_scope = HashMap::new();
    let mut generated_images_by_scope = HashMap::<_, HashMap<_, _>>::new();
    let mut frame_prompts_by_scope =
        HashMap::<(i64, i64), Vec<&chapter_video_prompt::Model>>::new();
    if !allow_downstream {
        for row in &rows {
            if prompt_stage(row) == Some(PromptStage::FramePrompt) {
                frame_prompts_by_scope
                    .entry((row.project_id, row.chapter_id))
                    .or_default()
                    .push(row);
            }
        }
    }
    for row in &rows {
        video::ensure_chapter_video_prompt_not_running(row, "删除视频提示词")?;
        if !allow_downstream {
            let Some(stage) = ensure_prompt_stage_mutability_preconditions(row, "删除")? else {
                continue;
            };
            if stage == PromptStage::FramePrompt {
                let scope = (row.project_id, row.chapter_id);
                let generated_images = if let Some(images) = generated_images_by_scope.get(&scope) {
                    images
                } else {
                    let frame_prompts = frame_prompts_by_scope
                        .get(&scope)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                    let images = generated_frame_images_for_prompts(
                        &txn,
                        row.project_id,
                        row.chapter_id,
                        frame_prompts,
                    )
                    .await?;
                    let mut by_prompt = HashMap::new();
                    for image in images {
                        if let Some(prompt_id) =
                            image_generation_prompt_id(image.metadata_json.as_ref())
                        {
                            by_prompt.entry(prompt_id).or_insert(image);
                        }
                    }
                    generated_images_by_scope.insert(scope, by_prompt);
                    generated_images_by_scope
                        .get(&scope)
                        .expect("generated frame images inserted for prompt scope")
                };
                ensure_no_generated_frame_image(row, "删除", generated_images.get(&row.id))?;
            }
            let scope = (row.project_id, row.chapter_id);
            match sibling_rows_by_scope.get(&scope) {
                Some(_) => {}
                None => {
                    let sibling_rows = video::chapter_video_prompt_dependency_records(
                        &txn,
                        row.project_id,
                        row.chapter_id,
                    )
                    .await
                    .map_err(VideoDbError::Database)?;
                    dependency_indexes_by_scope
                        .insert(scope, PromptDependencyIndex::build(&sibling_rows));
                    sibling_rows_by_scope.insert(scope, sibling_rows);
                }
            }
            let sibling_rows = sibling_rows_by_scope
                .get(&scope)
                .expect("sibling rows inserted for prompt scope");
            let dependency_index = dependency_indexes_by_scope
                .get(&scope)
                .expect("dependency index inserted for prompt scope");
            ensure_prompt_dependencies_mutable_with_index(
                row,
                "删除",
                &prompt_ids_set,
                stage,
                sibling_rows,
                dependency_index,
            )?;
        }
    }
    let deleted = video::delete_chapter_video_prompt_records(&txn, &rows).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(deleted)
}
