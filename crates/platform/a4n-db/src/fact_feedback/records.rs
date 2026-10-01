use a4n_db_models::models::{
    chapter::chapter,
    project::{material_fact_change, project_fact_feedback},
};
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::{
    fact_feedback::{DEFAULT_FEEDBACK_INTERVAL_CHAPTERS, FactSourceRevision, WorldFact},
    source_text_hash,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set, sea_query::OnConflict,
};
use serde_json::{Value, json};

use super::FactFeedbackDbError;

pub async fn feedback_config<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<project_fact_feedback::Model, DbErr> {
    Ok(project_fact_feedback::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .unwrap_or(project_fact_feedback::Model {
            project_id,
            interval_chapters: DEFAULT_FEEDBACK_INTERVAL_CHAPTERS,
            consolidated_sources_json: json!([]),
            last_consolidated_at: None,
        }))
}

pub async fn feedback_interval<C: ConnectionTrait>(db: &C, project_id: i64) -> Result<i32, DbErr> {
    Ok(feedback_config(db, project_id).await?.interval_chapters)
}

pub async fn feedback_intervals<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
) -> Result<std::collections::BTreeMap<i64, i32>, DbErr> {
    if project_ids.is_empty() {
        return Ok(Default::default());
    }
    Ok(project_fact_feedback::Entity::find()
        .filter(project_fact_feedback::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.project_id, row.interval_chapters))
        .collect())
}

pub async fn update_feedback_interval<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    interval: i32,
) -> Result<(), DbErr> {
    if project_id <= 0 || interval <= 0 {
        return Err(DbErr::Custom("项目 ID 和事实归纳周期必须是正整数".into()));
    }
    project_fact_feedback::Entity::insert(project_fact_feedback::ActiveModel {
        project_id: Set(project_id),
        interval_chapters: Set(interval),
        consolidated_sources_json: Set(json!([])),
        last_consolidated_at: Set(None),
    })
    .on_conflict(
        OnConflict::column(project_fact_feedback::Column::ProjectId)
            .update_column(project_fact_feedback::Column::IntervalChapters)
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}

pub fn source_revision(chapter: &ChapterRecord) -> Option<FactSourceRevision> {
    let analysis = chapter.analysis_data.as_ref()?;
    if analysis
        .get("_meta")
        .and_then(|meta| meta.get("fact_feedback_rebuild_required"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return None;
    }
    if !chapter.has_available_content() || analysis.get("world_facts").is_none() {
        return None;
    }
    if !analysis.is_object() {
        return None;
    }
    Some(observation_source(chapter))
}

pub(crate) fn observation_source(chapter: &ChapterRecord) -> FactSourceRevision {
    let mut analysis = chapter.analysis_data.clone().unwrap_or(Value::Null);
    if let Some(analysis) = analysis.as_object_mut() {
        analysis.remove("_meta");
    }
    FactSourceRevision {
        chapter_id: chapter.id,
        chapter_number: chapter.chapter_number,
        content_revision: a4n_domain::chapter::chapter_edit_revision(
            &chapter.content,
            chapter.title.as_deref(),
        ),
        analysis_hash: source_text_hash(&analysis.to_string()),
    }
}

#[derive(Debug, Clone)]
pub struct FeedbackSource {
    pub chapter: ChapterRecord,
    pub revision: FactSourceRevision,
    pub world_facts: Vec<WorldFact>,
}

pub async fn current_feedback_sources<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<Vec<FeedbackSource>, FactFeedbackDbError> {
    let chapters = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await?;
    let completed = material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .filter(material_fact_change::Column::MaterialKind.eq("chapter_feedback"))
        .filter(material_fact_change::Column::Status.eq("applied"))
        .all(db)
        .await?;
    let completed: std::collections::BTreeSet<_> = completed
        .into_iter()
        .filter_map(|record| {
            record
                .chapter_id
                .map(|id| (id, record.after_json.to_string()))
        })
        .collect();
    let mut result = Vec::new();
    for chapter in chapters {
        let chapter = crate::project::chapter_record(chapter);
        let Some(revision) = source_revision(&chapter) else {
            continue;
        };
        let revision_json = serde_json::to_value(&revision)?;
        if !completed.contains(&(chapter.id, revision_json.to_string())) {
            continue;
        }
        let world_facts: Vec<WorldFact> =
            serde_json::from_value(chapter.analysis_data.as_ref().unwrap()["world_facts"].clone())?;
        for fact in &world_facts {
            fact.validate().map_err(FactFeedbackDbError::Invalid)?;
        }
        result.push(FeedbackSource {
            chapter,
            revision,
            world_facts,
        });
    }
    Ok(result)
}

pub async fn feedback_changes<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<Vec<material_fact_change::Model>, DbErr> {
    material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .filter(material_fact_change::Column::MaterialKind.ne("chapter_feedback"))
        .order_by_desc(material_fact_change::Column::Id)
        .all(db)
        .await
}

pub struct FactChangeRecordInput {
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub material_kind: String,
    pub target_id: Option<i64>,
    pub field_path: String,
    pub operation: String,
    pub before: Value,
    pub after: Value,
    pub sources: Vec<FactSourceRevision>,
    pub evidence: Value,
    pub reason: String,
    pub status: String,
}

pub async fn record_fact_change<C: ConnectionTrait>(
    db: &C,
    input: FactChangeRecordInput,
) -> Result<material_fact_change::Model, FactFeedbackDbError> {
    if input.sources.is_empty()
        || input
            .sources
            .iter()
            .any(|source| source.chapter_id <= 0 || source.chapter_number <= 0)
    {
        return Err(FactFeedbackDbError::Invalid(
            "资料变更缺少有效正文来源".into(),
        ));
    }
    let sources = serde_json::to_value(&input.sources)?;
    let key = source_text_hash(
        &json!([
            input.material_kind,
            input.target_id,
            input.field_path,
            input.operation,
            input.before,
            input.after,
            sources,
            input.status
        ])
        .to_string(),
    );
    if let Some(existing) = material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(input.project_id))
        .filter(material_fact_change::Column::ChangeKey.eq(&key))
        .one(db)
        .await?
    {
        if existing.status != "retracted" {
            return Ok(existing);
        }
        // A reapplication is a new event in the undo order; keep the withdrawn event as history.
        let retired_key = format!("{key}:retracted:{}", existing.id);
        let mut active: material_fact_change::ActiveModel = existing.into();
        active.change_key = Set(retired_key);
        active.update(db).await?;
    }
    Ok(material_fact_change::ActiveModel {
        project_id: Set(input.project_id),
        workflow_run_id: Set(input.workflow_run_id),
        chapter_id: Set((input.sources.len() == 1).then_some(input.sources[0].chapter_id)),
        change_key: Set(key),
        material_kind: Set(input.material_kind),
        target_id: Set(input.target_id),
        field_path: Set(input.field_path),
        operation: Set(input.operation),
        before_json: Set(input.before),
        after_json: Set(input.after),
        source_refs_json: Set(sources),
        evidence_json: Set(input.evidence),
        reason: Set(input.reason),
        status: Set(input.status),
        created_at: Set(Utc::now().naive_utc()),
        invalidated_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

pub async fn record_feedback_completion(
    db: &DatabaseTransaction,
    chapter: &ChapterRecord,
    workflow_run_id: i64,
) -> Result<(), FactFeedbackDbError> {
    let source = source_revision(chapter).ok_or_else(|| {
        FactFeedbackDbError::Invalid("旧版章节分析需要重新分析以补齐正文事实".into())
    })?;
    record_fact_change(
        db,
        FactChangeRecordInput {
            project_id: chapter.project_id,
            workflow_run_id: Some(workflow_run_id),
            material_kind: "chapter_feedback".into(),
            target_id: Some(chapter.id),
            field_path: String::new(),
            operation: "replace".into(),
            before: Value::Null,
            after: serde_json::to_value(&source)?,
            sources: vec![source],
            evidence: json!([]),
            reason: "逐章事实反馈完成".into(),
            status: "applied".into(),
        },
    )
    .await?;
    Ok(())
}
