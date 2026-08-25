use crate::project_dependency_graph::AssetLineageRecord;
use crate::project_dependency_graph::details::records::{db_record_result, lineage_ref};
use a4n_core::{error::AppError, models::generation::generation_spec};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project_dependency_graph) async fn generation_spec_detail_records<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(specs.into_iter().map(generation_spec_record).collect())
}

pub(super) fn generation_spec_record(spec: generation_spec::Model) -> AssetLineageRecord {
    AssetLineageRecord {
        id: format!("generation_spec:{}", spec.id),
        label: format!("{} #{}", spec.target_kind, spec.id),
        asset_kind: "generation_spec".to_string(),
        asset_id: Some(spec.id),
        status: Some(spec.status.clone()),
        origin: "llm_generated".to_string(),
        source_refs: spec
            .source_chapter_id
            .map(|chapter_id| {
                vec![lineage_ref(
                    &format!("chapter:{chapter_id}"),
                    "源章节",
                    "text",
                    "chapter",
                    Some(chapter_id),
                )]
            })
            .unwrap_or_default(),
        prompt_sources: Vec::new(),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: if spec.source_chapter_id.is_none()
            || spec.source_snapshot_chapter_id.is_some()
        {
            "complete"
        } else {
            "missing"
        }
        .to_string(),
        generation_inputs: Vec::new(),
        result: db_record_result("generation_spec", &spec),
        warnings: if spec.source_chapter_id.is_some() && spec.source_snapshot_chapter_id.is_none() {
            vec!["生成规格缺少 source_snapshot_chapter_id，源快照章节链路不完整。".to_string()]
        } else {
            Vec::new()
        },
    }
}
