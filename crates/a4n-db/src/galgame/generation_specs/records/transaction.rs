use super::query::galgame_generation_specs_by_canonical_key;
use super::references::replace_galgame_generation_spec_ref_records;
use super::upsert::upsert_galgame_generation_spec_record;
use crate::galgame::{
    GalgameAssetPlanSpecRecordError, GalgameAssetPlanSpecRecordInput,
    GalgameGenerationSpecEntityRefRecordInput, GalgameGenerationSpecRefsRecordInput,
    GalgameGenerationSpecTextAnchorRefRecordInput, GalgameGenerationSpecUpsert,
};
use sea_orm::{DatabaseConnection, TransactionTrait};
use std::collections::BTreeSet;

pub async fn upsert_galgame_asset_plan_spec_records_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    inputs: Vec<GalgameAssetPlanSpecRecordInput>,
) -> Result<Vec<GalgameGenerationSpecUpsert>, GalgameAssetPlanSpecRecordError> {
    let tx = db.begin().await?;
    let mut existing_specs_by_canonical_key =
        galgame_generation_specs_by_canonical_key(&tx, project_id).await?;
    let mut refs = GalgameGenerationSpecRefsRecordInput::default();
    let mut upserts = Vec::with_capacity(inputs.len());

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
        let upsert = upsert_galgame_generation_spec_record(
            &tx,
            &mut existing_specs_by_canonical_key,
            input.spec,
        )
        .await?;

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
        upserts.push(upsert);
    }

    replace_galgame_generation_spec_ref_records(&tx, refs).await?;
    tx.commit().await?;
    Ok(upserts)
}
