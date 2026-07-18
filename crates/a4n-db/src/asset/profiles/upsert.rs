use std::collections::BTreeSet;

use a4n_core::models::asset::object_profile;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::{Value, json};

use crate::asset::{AssetDbError, ObjectProfileRecordInput, ObjectProfileUpsertRecordResult};

use super::read::object_profile_by_normalized_name;
use super::record::{apply_object_profile_input, insert_object_profile_record};

pub(in crate::asset) async fn upsert_object_profiles_in<C>(
    db: &C,
    project_id: i64,
    inputs: Vec<ObjectProfileRecordInput>,
) -> Result<ObjectProfileUpsertRecordResult, AssetDbError>
where
    C: ConnectionTrait,
{
    let mut result = ObjectProfileUpsertRecordResult::default();
    let mut seen = BTreeSet::new();
    for input in inputs {
        if !seen.insert(input.normalized_name.clone()) {
            return Err(AssetDbError::InvalidArgument(format!(
                "物体发现结果存在重复物体: {}",
                input.name
            )));
        }
        match object_profile_by_normalized_name(db, project_id, &input.normalized_name).await? {
            Some(model) => result
                .updated
                .push(merge_object_profile_record(db, model, input).await?),
            None => result
                .created
                .push(insert_object_profile_record(db, project_id, input).await?),
        }
    }
    Ok(result)
}

async fn merge_object_profile_record<C>(
    db: &C,
    existing: object_profile::Model,
    input: ObjectProfileRecordInput,
) -> Result<object_profile::Model, AssetDbError>
where
    C: ConnectionTrait,
{
    let first_seen_chapter = existing.first_seen_chapter.min(input.first_seen_chapter);
    let last_seen_chapter = existing.last_seen_chapter.max(input.last_seen_chapter);
    let evidence_json =
        merge_text_set_json(existing.evidence_json.clone(), input.evidence_json.clone());
    let mut active: object_profile::ActiveModel = existing.into();
    apply_object_profile_input(&mut active, input);
    active.first_seen_chapter = Set(first_seen_chapter);
    active.last_seen_chapter = Set(last_seen_chapter);
    active.evidence_json = Set(evidence_json);
    active.update(db).await.map_err(AssetDbError::Database)
}

fn merge_text_set_json(existing: Value, incoming: Value) -> Value {
    let mut values = existing
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    values.extend(
        incoming
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    );
    json!(values.into_iter().collect::<Vec<_>>())
}
