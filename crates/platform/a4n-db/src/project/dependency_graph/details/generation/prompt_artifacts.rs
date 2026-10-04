use std::collections::HashMap;

use a4n_db_models::models::generation::{generation_spec, prompt_artifact};
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::{
    db_record_result_with_related, db_related_record, lineage_ref, optional_prompt_sources,
    prompt_source,
};

pub(in crate::project::dependency_graph) async fn prompt_artifact_detail_records<C>(
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
        .map_err(AppError::from)?;
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let spec_by_id = specs
        .iter()
        .map(|spec| (spec.id, spec))
        .collect::<HashMap<_, _>>();
    let artifacts = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(specs.iter().map(|spec| spec.id)))
        .order_by_asc(prompt_artifact::Column::Id)
        .all(db)
        .await
        .map_err(AppError::from)?;

    Ok(artifacts
        .into_iter()
        .map(|artifact| {
            let spec = spec_by_id.get(&artifact.generation_spec_id).copied();
            prompt_artifact_record(artifact, spec)
        })
        .collect())
}

pub(super) fn prompt_artifact_record(
    artifact: prompt_artifact::Model,
    spec: Option<&generation_spec::Model>,
) -> AssetLineageRecord {
    AssetLineageRecord {
        id: format!("prompt_artifact:{}", artifact.id),
        label: format!("{} #{}", artifact.prompt_kind, artifact.id),
        asset_kind: "prompt_artifact".to_string(),
        asset_id: Some(artifact.id),
        status: None,
        origin: "llm_generated".to_string(),
        source_refs: spec
            .map(|spec| {
                vec![lineage_ref(
                    &format!("generation_spec:{}", spec.id),
                    "统一生成规格",
                    "text",
                    "generation_spec",
                    Some(spec.id),
                )]
            })
            .unwrap_or_default(),
        prompt_sources: optional_prompt_sources(vec![prompt_source(
            &format!("prompt_artifact_text:{}", artifact.id),
            "prompt_artifact",
            Some(&artifact.prompt_text),
            Some(artifact.id),
            Some(artifact.edited_by_user),
        )]),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: if spec.is_some() {
            "complete"
        } else {
            "missing"
        }
        .to_string(),
        generation_inputs: Vec::new(),
        result: db_record_result_with_related(
            "prompt_artifact",
            &artifact,
            spec.map(|spec| db_related_record("generation_spec", spec))
                .into_iter()
                .collect(),
        ),
        warnings: if spec.is_none() {
            vec!["提示词产物指向的 generation_spec 不存在。".to_string()]
        } else {
            Vec::new()
        },
    }
}
