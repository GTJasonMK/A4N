use super::query::galgame_generation_specs_by_canonical_key;
use super::references::replace_galgame_generation_spec_ref_records;
use super::upsert::{
    galgame_generation_spec_content_changed, new_galgame_generation_spec_model,
    updated_galgame_generation_spec_model,
};
use crate::galgame::{
    GalgameAssetPlanSpecRecordError, GalgameAssetPlanSpecRecordInput,
    GalgameGenerationSpecEntityRefRecordInput, GalgameGenerationSpecRefsRecordInput,
    GalgameGenerationSpecTextAnchorRefRecordInput, GalgameGenerationSpecUpsert,
    GalgameGenerationSpecUpsertOutcome,
};
use a4n_db_models::models::{generation::generation_spec, project::project_source_snapshot};
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter, QuerySelect,
    TransactionTrait,
};
use std::collections::{BTreeMap, BTreeSet};

pub async fn upsert_galgame_asset_plan_spec_records_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    inputs: Vec<GalgameAssetPlanSpecRecordInput>,
) -> Result<Vec<GalgameGenerationSpecUpsert>, GalgameAssetPlanSpecRecordError> {
    let tx = db.begin().await?;
    let mut existing_specs_by_canonical_key =
        galgame_generation_specs_by_canonical_key(&tx, project_id).await?;
    let mut upserts = Vec::with_capacity(inputs.len());
    let mut result_keys = Vec::with_capacity(inputs.len());
    let mut ref_inputs = Vec::with_capacity(inputs.len());
    let mut pending_inserts = BTreeMap::<String, generation_spec::Model>::new();
    let mut pending_updates = BTreeMap::<i64, generation_spec::Model>::new();
    let snapshots = project_source_snapshot::Entity::find()
        .select_only()
        .columns([
            project_source_snapshot::Column::Id,
            project_source_snapshot::Column::RootProjectId,
            project_source_snapshot::Column::SourceVersionHash,
        ])
        .filter(
            project_source_snapshot::Column::Id.is_in(
                inputs
                    .iter()
                    .map(|input| input.spec.source_snapshot_id)
                    .collect::<BTreeSet<_>>(),
            ),
        )
        .into_tuple::<(i64, i64, String)>()
        .all(&tx)
        .await?
        .into_iter()
        .map(|(id, root, hash)| (id, (root, hash)))
        .collect::<BTreeMap<_, _>>();

    for input in inputs {
        if input.spec.project_id != project_id {
            return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(format!(
                "Galgame 资产生成意图项目不匹配: expected_project_id={}, actual_project_id={}",
                project_id, input.spec.project_id
            )));
        }
        let text_anchor_ids = input.text_anchor_ids.into_iter().collect::<BTreeSet<_>>();
        if text_anchor_ids.is_empty() {
            return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(
                "Galgame 资产生成意图必须至少绑定一个文本锚点".to_string(),
            ));
        }
        let replace_entity_refs = input.replace_entity_refs;
        let entity_profile_id = input.entity_profile_id;
        let spec = input.spec;
        let canonical_key = spec.canonical_key.clone();
        let (_, source_version_hash) = snapshots
            .get(&spec.source_snapshot_id)
            .filter(|(root, _)| *root == spec.root_project_id)
            .ok_or_else(|| {
                GalgameAssetPlanSpecRecordError::InvalidArgument(format!(
                    "Galgame 资产生成意图的源快照不属于根项目: snapshot_id={}, root_project_id={}",
                    spec.source_snapshot_id, spec.root_project_id
                ))
            })?;
        let source_version_hash = source_version_hash.clone();
        let now = Utc::now().naive_utc();
        let (model, outcome) = if let Some(existing) =
            existing_specs_by_canonical_key.get(&canonical_key).cloned()
        {
            if !galgame_generation_spec_content_changed(&existing, &spec, &source_version_hash) {
                (existing, GalgameGenerationSpecUpsertOutcome::Reused)
            } else {
                let updated = updated_galgame_generation_spec_model(
                    &existing,
                    spec,
                    source_version_hash,
                    now,
                );
                if updated.id > 0 {
                    pending_updates.insert(updated.id, updated.clone());
                } else {
                    pending_inserts.insert(canonical_key.clone(), updated.clone());
                }
                (updated, GalgameGenerationSpecUpsertOutcome::Updated)
            }
        } else {
            let created = new_galgame_generation_spec_model(spec, source_version_hash, now);
            pending_inserts.insert(canonical_key.clone(), created.clone());
            (created, GalgameGenerationSpecUpsertOutcome::Created)
        };
        existing_specs_by_canonical_key.insert(canonical_key.clone(), model.clone());
        result_keys.push(canonical_key);
        ref_inputs.push((text_anchor_ids, replace_entity_refs, entity_profile_id));
        upserts.push(GalgameGenerationSpecUpsert { model, outcome });
    }

    if !pending_inserts.is_empty() {
        let insert_keys = pending_inserts.keys().cloned().collect::<Vec<_>>();
        let inserts = pending_inserts
            .into_values()
            .map(|model| {
                let mut active: generation_spec::ActiveModel = model.into();
                active.id = NotSet;
                active
            })
            .collect::<Vec<_>>();
        let inserted = generation_spec::Entity::insert_many(inserts)
            .exec_with_returning_many(&tx)
            .await?;
        if inserted.len() != insert_keys.len() {
            return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(format!(
                "Galgame 资产生成意图批量插入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                insert_keys.len()
            )));
        }
        let inserted_by_key = insert_keys
            .into_iter()
            .zip(inserted)
            .collect::<BTreeMap<_, _>>();
        for (upsert, key) in upserts.iter_mut().zip(&result_keys) {
            if upsert.model.id == 0 {
                upsert.model.id = inserted_by_key
                    .get(key)
                    .expect("inserted generation spec key must exist")
                    .id;
            }
        }
    }

    if !pending_updates.is_empty() {
        let ids = pending_updates.keys().copied().collect::<Vec<_>>();
        let mut root_project_id_case = CaseStatement::new();
        let mut source_context_kind_case = CaseStatement::new();
        let mut source_snapshot_id_case = CaseStatement::new();
        let mut source_chapter_id_case = CaseStatement::new();
        let mut source_snapshot_chapter_id_case = CaseStatement::new();
        let mut canvas_text_asset_id_case = CaseStatement::new();
        let mut target_kind_case = CaseStatement::new();
        let mut target_entity_kind_case = CaseStatement::new();
        let mut target_entity_id_case = CaseStatement::new();
        let mut status_case = CaseStatement::new();
        let mut source_version_hash_case = CaseStatement::new();
        let mut params_json_case = CaseStatement::new();
        let mut updated_at_case = CaseStatement::new();
        for (id, model) in &pending_updates {
            root_project_id_case = root_project_id_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.root_project_id),
            );
            source_context_kind_case = source_context_kind_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.source_context_kind.clone()),
            );
            source_snapshot_id_case = source_snapshot_id_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.source_snapshot_id),
            );
            source_chapter_id_case = source_chapter_id_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.source_chapter_id),
            );
            source_snapshot_chapter_id_case = source_snapshot_chapter_id_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.source_snapshot_chapter_id),
            );
            canvas_text_asset_id_case = canvas_text_asset_id_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.canvas_text_asset_id),
            );
            target_kind_case = target_kind_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.target_kind.clone()),
            );
            target_entity_kind_case = target_entity_kind_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.target_entity_kind.clone()),
            );
            target_entity_id_case = target_entity_id_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.target_entity_id),
            );
            status_case = status_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.status.clone()),
            );
            source_version_hash_case = source_version_hash_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.source_version_hash.clone()),
            );
            params_json_case = params_json_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.params_json.clone()).cast_as("json"),
            );
            updated_at_case = updated_at_case.case(
                generation_spec::Column::Id.eq(*id),
                Expr::value(model.updated_at),
            );
        }
        let updated = generation_spec::Entity::update_many()
            .col_expr(
                generation_spec::Column::RootProjectId,
                root_project_id_case
                    .finally(Expr::col(generation_spec::Column::RootProjectId))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::SourceContextKind,
                source_context_kind_case
                    .finally(Expr::col(generation_spec::Column::SourceContextKind))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::SourceSnapshotId,
                source_snapshot_id_case
                    .finally(Expr::col(generation_spec::Column::SourceSnapshotId))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::SourceChapterId,
                source_chapter_id_case
                    .finally(Expr::col(generation_spec::Column::SourceChapterId))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::SourceSnapshotChapterId,
                source_snapshot_chapter_id_case
                    .finally(Expr::col(generation_spec::Column::SourceSnapshotChapterId))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::CanvasTextAssetId,
                canvas_text_asset_id_case
                    .finally(Expr::col(generation_spec::Column::CanvasTextAssetId))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::TargetKind,
                target_kind_case
                    .finally(Expr::col(generation_spec::Column::TargetKind))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::TargetEntityKind,
                target_entity_kind_case
                    .finally(Expr::col(generation_spec::Column::TargetEntityKind))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::TargetEntityId,
                target_entity_id_case
                    .finally(Expr::col(generation_spec::Column::TargetEntityId))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::Status,
                status_case
                    .finally(Expr::col(generation_spec::Column::Status))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::SourceVersionHash,
                source_version_hash_case
                    .finally(Expr::col(generation_spec::Column::SourceVersionHash))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::ParamsJson,
                params_json_case
                    .finally(Expr::col(generation_spec::Column::ParamsJson))
                    .into(),
            )
            .col_expr(
                generation_spec::Column::UpdatedAt,
                updated_at_case
                    .finally(Expr::col(generation_spec::Column::UpdatedAt))
                    .into(),
            )
            .filter(generation_spec::Column::ProjectId.eq(project_id))
            .filter(generation_spec::Column::Id.is_in(ids.clone()))
            .exec(&tx)
            .await?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(format!(
                "Galgame 资产生成意图批量更新返回行数不一致: updated={}, expected={}",
                updated.rows_affected,
                ids.len()
            )));
        }
    }

    let mut refs = GalgameGenerationSpecRefsRecordInput::default();
    for ((text_anchor_ids, replace_entity_refs, entity_profile_id), upsert) in
        ref_inputs.into_iter().zip(&upserts)
    {
        refs.text_anchor_spec_ids.push(upsert.model.id);
        refs.text_anchor_refs
            .extend(text_anchor_ids.into_iter().map(|text_anchor_id| {
                GalgameGenerationSpecTextAnchorRefRecordInput {
                    generation_spec_id: upsert.model.id,
                    text_anchor_id,
                }
            }));
        if replace_entity_refs {
            refs.entity_ref_spec_ids.push(upsert.model.id);
            if let Some(profile_id) = entity_profile_id {
                refs.entity_refs
                    .push(GalgameGenerationSpecEntityRefRecordInput {
                        generation_spec_id: upsert.model.id,
                        profile_id,
                    });
            }
        }
    }

    replace_galgame_generation_spec_ref_records(&tx, refs).await?;
    tx.commit().await?;
    Ok(upserts)
}
