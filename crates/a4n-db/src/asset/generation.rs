use super::contracts::{AssetDbError, ObjectProfileRecordInput, ObjectProfileUpsertRecordResult};
use super::profiles::{ensure_object_profile_project_exists, upsert_object_profiles_in};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::{Value, json};

pub async fn upsert_object_profiles_with_generation_run_record<C>(
    db: &C,
    project_id: i64,
    inputs: Vec<ObjectProfileRecordInput>,
    mut generation_run: crate::generation::RecordCompletedGenerationRunInput,
) -> Result<ObjectProfileUpsertRecordResult, AssetDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if generation_run.project_id != project_id {
        return Err(AssetDbError::InvalidArgument(
            "物体档案生成运行记录 project_id 与项目不一致".to_string(),
        ));
    }
    if !generation_run.targets.is_empty() {
        return Err(AssetDbError::InvalidArgument(
            "物体档案生成运行记录目标由物体服务绑定，调用方不能预填 targets".to_string(),
        ));
    }

    let txn = db.begin().await?;
    ensure_object_profile_project_exists(&txn, project_id).await?;
    let result = upsert_object_profiles_in(&txn, project_id, inputs).await?;
    generation_run.targets = result
        .created
        .iter()
        .chain(result.updated.iter())
        .map(|profile| crate::generation::GenerationRunTraceTarget {
            target_kind: "object_profile_record".to_string(),
            target_table: "object_profile".to_string(),
            target_id: profile.id,
        })
        .collect();
    if !generation_run.targets.is_empty() {
        generation_run.response_summary_json = object_profile_generation_response_summary(
            generation_run.response_summary_json,
            &result,
        );
        crate::generation::record_completed_generation_run(&txn, generation_run).await?;
    }
    txn.commit().await?;
    Ok(result)
}

fn object_profile_generation_response_summary(
    summary: Option<Value>,
    result: &ObjectProfileUpsertRecordResult,
) -> Option<Value> {
    let generated = json!({
        "created_count": result.created.len(),
        "updated_count": result.updated.len(),
        "profile_ids": result
            .created
            .iter()
            .chain(result.updated.iter())
            .map(|profile| profile.id)
            .collect::<Vec<_>>(),
    });
    match summary {
        Some(Value::Object(mut map)) => {
            if let Some(generated) = generated.as_object() {
                for (key, value) in generated {
                    map.insert(key.clone(), value.clone());
                }
            }
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "object_profiles": generated,
        })),
        None => Some(generated),
    }
}
