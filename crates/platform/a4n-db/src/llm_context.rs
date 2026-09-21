use a4n_db_models::models::generation::llm_context_pack;
use a4n_domain::llm::{ContextPackReference, RecordOrReuseContextPackInput};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect, Set,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, thiserror::Error)]
pub enum LlmContextPackDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("数据库错误: {0}")]
    Database(#[from] DbErr),
}

pub async fn record_or_reuse_context_packs<C>(
    db: &C,
    inputs: Vec<RecordOrReuseContextPackInput>,
) -> Result<Vec<llm_context_pack::Model>, LlmContextPackDbError>
where
    C: ConnectionTrait,
{
    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    let lookup = context_pack_lookup_condition(&inputs);
    let mut models = llm_context_pack::Entity::find()
        .filter(lookup)
        .all(db)
        .await?
        .into_iter()
        .map(|model| (identity_key_model(&model), model))
        .collect::<HashMap<_, _>>();

    let existing_keys = models.keys().cloned().collect::<HashSet<_>>();
    let pending_inputs = pending_context_pack_inputs(&existing_keys, &inputs);
    if !pending_inputs.is_empty() {
        let inserted = insert_context_packs(db, pending_inputs).await?;
        for model in inserted {
            models.insert(identity_key_model(&model), model);
        }
    }

    inputs_for_models(&models, &inputs)
}

pub async fn record_or_reuse_context_pack_reference<C>(
    db: &C,
    input: RecordOrReuseContextPackInput,
) -> Result<ContextPackReference, LlmContextPackDbError>
where
    C: ConnectionTrait,
{
    record_or_reuse_context_pack_references(db, vec![input])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            LlmContextPackDbError::InvalidArgument("LLM 上下文包输入不能为空".to_string())
        })
}

pub async fn record_or_reuse_context_pack_references<C>(
    db: &C,
    inputs: Vec<RecordOrReuseContextPackInput>,
) -> Result<Vec<ContextPackReference>, LlmContextPackDbError>
where
    C: ConnectionTrait,
{
    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    let lookup = context_pack_lookup_condition(&inputs);
    let mut references = llm_context_pack::Entity::find()
        .select_only()
        .column(llm_context_pack::Column::Id)
        .column(llm_context_pack::Column::ProjectId)
        .column(llm_context_pack::Column::Workflow)
        .column(llm_context_pack::Column::PackKind)
        .column(llm_context_pack::Column::CompressionLevel)
        .column(llm_context_pack::Column::ContentSha256)
        .column(llm_context_pack::Column::ConfigHash)
        .column(llm_context_pack::Column::TokenEstimate)
        .filter(lookup)
        .into_tuple::<(i64, i64, String, String, String, String, String, i32)>()
        .all(db)
        .await?
        .into_iter()
        .map(
            |(
                id,
                project_id,
                workflow,
                pack_kind,
                compression_level,
                content_sha256,
                config_hash,
                token_estimate,
            )| {
                (
                    (
                        project_id,
                        workflow,
                        pack_kind.clone(),
                        compression_level.clone(),
                        content_sha256.clone(),
                        config_hash,
                    ),
                    ContextPackReference {
                        id,
                        pack_kind,
                        content_sha256,
                        compression_level,
                        token_estimate,
                    },
                )
            },
        )
        .collect::<HashMap<_, _>>();

    let existing_keys = references.keys().cloned().collect::<HashSet<_>>();
    let pending_inputs = pending_context_pack_inputs(&existing_keys, &inputs);
    if !pending_inputs.is_empty() {
        let inserted = insert_context_packs(db, pending_inputs).await?;
        for model in inserted {
            references.insert(
                identity_key_model(&model),
                context_pack_reference_from_model(&model),
            );
        }
    }

    inputs_for_references(&references, &inputs)
}

fn inputs_for_models(
    models: &HashMap<ContextPackIdentity, llm_context_pack::Model>,
    inputs: &[RecordOrReuseContextPackInput],
) -> Result<Vec<llm_context_pack::Model>, LlmContextPackDbError> {
    inputs
        .iter()
        .map(|input| {
            models
                .get(&identity_key_input(input))
                .cloned()
                .ok_or_else(|| {
                    LlmContextPackDbError::InvalidArgument(
                        "LLM 上下文包批量读取缺少对应记录".to_string(),
                    )
                })
        })
        .collect()
}

type ContextPackIdentity = (i64, String, String, String, String, String);

fn identity_key_input(input: &RecordOrReuseContextPackInput) -> ContextPackIdentity {
    (
        input.project_id,
        input.workflow.clone(),
        input.pack_kind.clone(),
        input.compression_level.clone(),
        input.content_sha256.clone(),
        input.config_hash.clone(),
    )
}

fn context_pack_lookup_condition(inputs: &[RecordOrReuseContextPackInput]) -> Condition {
    inputs.iter().fold(Condition::any(), |condition, input| {
        condition.add(
            Condition::all()
                .add(llm_context_pack::Column::ProjectId.eq(input.project_id))
                .add(llm_context_pack::Column::Workflow.eq(input.workflow.as_str()))
                .add(llm_context_pack::Column::PackKind.eq(input.pack_kind.as_str()))
                .add(
                    llm_context_pack::Column::CompressionLevel.eq(input.compression_level.as_str()),
                )
                .add(llm_context_pack::Column::ContentSha256.eq(input.content_sha256.as_str()))
                .add(llm_context_pack::Column::ConfigHash.eq(input.config_hash.as_str())),
        )
    })
}

fn pending_context_pack_inputs(
    existing_keys: &HashSet<ContextPackIdentity>,
    inputs: &[RecordOrReuseContextPackInput],
) -> Vec<RecordOrReuseContextPackInput> {
    let mut pending_keys = HashSet::new();
    inputs
        .iter()
        .filter(|input| {
            let key = identity_key_input(input);
            !existing_keys.contains(&key) && pending_keys.insert(key)
        })
        .cloned()
        .collect()
}

async fn insert_context_packs<C>(
    db: &C,
    pending_inputs: Vec<RecordOrReuseContextPackInput>,
) -> Result<Vec<llm_context_pack::Model>, LlmContextPackDbError>
where
    C: ConnectionTrait,
{
    let expected = pending_inputs.len();
    let inserted = llm_context_pack::Entity::insert_many(
        pending_inputs
            .into_iter()
            .map(context_pack_active_model)
            .collect::<Result<Vec<_>, _>>()?,
    )
    .exec_with_returning_many(db)
    .await?;
    if inserted.len() != expected {
        return Err(LlmContextPackDbError::InvalidArgument(format!(
            "LLM 上下文包批量插入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            expected
        )));
    }
    Ok(inserted)
}

fn identity_key_model(model: &llm_context_pack::Model) -> ContextPackIdentity {
    (
        model.project_id,
        model.workflow.clone(),
        model.pack_kind.clone(),
        model.compression_level.clone(),
        model.content_sha256.clone(),
        model.config_hash.clone(),
    )
}

fn context_pack_reference_from_model(model: &llm_context_pack::Model) -> ContextPackReference {
    ContextPackReference {
        id: model.id,
        pack_kind: model.pack_kind.clone(),
        content_sha256: model.content_sha256.clone(),
        compression_level: model.compression_level.clone(),
        token_estimate: model.token_estimate,
    }
}

fn inputs_for_references(
    references: &HashMap<ContextPackIdentity, ContextPackReference>,
    inputs: &[RecordOrReuseContextPackInput],
) -> Result<Vec<ContextPackReference>, LlmContextPackDbError> {
    inputs
        .iter()
        .map(|input| {
            references
                .get(&identity_key_input(input))
                .cloned()
                .ok_or_else(|| {
                    LlmContextPackDbError::InvalidArgument(
                        "LLM 上下文包批量读取缺少对应记录".to_string(),
                    )
                })
        })
        .collect()
}

fn context_pack_active_model(
    input: RecordOrReuseContextPackInput,
) -> Result<llm_context_pack::ActiveModel, LlmContextPackDbError> {
    let now = Utc::now().naive_utc();
    Ok(llm_context_pack::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        scope_kind: Set(input.scope_kind),
        workflow: Set(input.workflow),
        pack_kind: Set(input.pack_kind),
        source_refs_json: Set(input.source_refs_json),
        config_hash: Set(input.config_hash),
        content_sha256: Set(input.content_sha256),
        canonical_content_json: Set(input.canonical_content_json),
        canonical_content_text: Set(input.canonical_content_text),
        compression_level: Set(input.compression_level),
        token_estimate: Set(usize_to_i32(input.token_estimate, "token_estimate")?),
        created_at: Set(now),
        updated_at: Set(now),
    })
}

fn usize_to_i32(value: usize, field: &str) -> Result<i32, LlmContextPackDbError> {
    i32::try_from(value).map_err(|_| {
        LlmContextPackDbError::InvalidArgument(format!("LLM 上下文字段 {field} 超出 i32 范围"))
    })
}
