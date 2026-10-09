use a4n_db_models::models::chapter::chapter::{self, STATUS_DRAFT};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, QueryFilter,
    QuerySelect, Set, Statement, TransactionTrait,
};

use super::{
    ChapterGenerationExistingRecord, ChapterGenerationLifecycleError,
    GeneratedChapterContentRecordInput, assign_chapter_content,
};
use crate::DbHandle;
use crate::generation::delete_generation_run_targets_for;

pub async fn begin_chapter_generation_record(
    db: &DbHandle,
    project_id: i64,
    chapter_number: i32,
    task_id: &str,
    now: NaiveDateTime,
) -> Result<Option<ChapterGenerationExistingRecord>, ChapterGenerationLifecycleError> {
    let tx = db
        .connection()
        .begin()
        .await
        .map_err(ChapterGenerationLifecycleError::database)?;
    let generation_scope = crate::generation::chapter_generation_project_lock_scope(project_id);
    tx.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        [generation_scope.into()],
    ))
    .await
    .map_err(ChapterGenerationLifecycleError::database)?;

    let generating_chapter_number = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.eq(chapter::STATUS_GENERATING))
        .filter(chapter::Column::ChapterNumber.ne(chapter_number))
        .select_only()
        .column(chapter::Column::ChapterNumber)
        .into_tuple::<i32>()
        .one(&tx)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?;
    if let Some(generating_chapter_number) = generating_chapter_number {
        return Err(ChapterGenerationLifecycleError::InvalidState {
            current: chapter::STATUS_GENERATING.to_string(),
            operation: format!("生成第{chapter_number}章；第{generating_chapter_number}章正在生成"),
        });
    }

    let active_chapter_tasks = tx
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT checkpoint.task_id \
             FROM generation_task_checkpoint AS checkpoint \
             JOIN task_queue AS queue ON queue.id = checkpoint.task_id \
             WHERE queue.project_id = $1 \
               AND queue.task_scope = 'chapter_generation' \
               AND queue.status IN ('pending', 'running') \
               AND checkpoint.status IN ('reserved', 'running', 'finalizing', 'cancelling') \
             ORDER BY queue.created_at, checkpoint.task_id",
            [project_id.into()],
        ))
        .await
        .map_err(ChapterGenerationLifecycleError::database)?;
    let first_task = active_chapter_tasks.first().ok_or_else(|| {
        ChapterGenerationLifecycleError::InvalidState {
            current: "missing_active_chapter_generation_task".to_string(),
            operation: format!("开始生成第{chapter_number}章"),
        }
    })?;
    let first_task_id: String = first_task
        .try_get("", "task_id")
        .map_err(ChapterGenerationLifecycleError::database)?;
    if first_task_id != task_id {
        return Err(ChapterGenerationLifecycleError::InvalidState {
            current: "another_chapter_generation_task_is_ahead_in_queue".to_string(),
            operation: format!("生成第{chapter_number}章；请等待较早排队的章节生成任务结束"),
        });
    }

    let Some((chapter_id, status, title, summary)) = chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::Status)
        .column(chapter::Column::Title)
        .column(chapter::Column::Summary)
        .into_tuple::<(i64, String, Option<String>, Option<String>)>()
        .one(&tx)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?
    else {
        tx.commit()
            .await
            .map_err(ChapterGenerationLifecycleError::database)?;
        return Ok(None);
    };

    if status == chapter::STATUS_GENERATING {
        return Err(ChapterGenerationLifecycleError::InvalidState {
            current: status,
            operation: format!("生成第{chapter_number}章"),
        });
    }
    let previous_status = status;

    chapter::ActiveModel {
        id: Set(chapter_id),
        status: Set(chapter::STATUS_GENERATING.to_string()),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(&tx)
    .await
    .map_err(ChapterGenerationLifecycleError::database)?;
    tx.commit()
        .await
        .map_err(ChapterGenerationLifecycleError::database)?;
    Ok(Some(ChapterGenerationExistingRecord {
        id: chapter_id,
        previous_status,
        title,
        summary,
    }))
}

pub async fn restore_chapter_generation_status_record(
    db: &DbHandle,
    chapter_id: i64,
    previous_status: String,
    now: NaiveDateTime,
) -> Result<bool, ChapterGenerationLifecycleError> {
    let db = db.connection();
    let Some(status) = chapter::Entity::find_by_id(chapter_id)
        .select_only()
        .column(chapter::Column::Status)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(ChapterGenerationLifecycleError::database)?
    else {
        return Ok(false);
    };

    if status != chapter::STATUS_GENERATING {
        return Ok(false);
    }

    chapter::ActiveModel {
        id: Set(chapter_id),
        status: Set(previous_status),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await
    .map_err(ChapterGenerationLifecycleError::database)?;
    Ok(true)
}

/// Writes the canonical chapter result into an already-open transaction.
///
/// Generation terminal completion uses this entry so the chapter row and the
/// generation terminal/checkpoint commit share one database transaction.
pub async fn finish_generated_chapter_content_record_in_transaction<C>(
    db: &C,
    input: GeneratedChapterContentRecordInput,
) -> Result<a4n_domain::chapter::ChapterRecord, ChapterGenerationLifecycleError>
where
    C: ConnectionTrait,
{
    let outline_id = match input.source_outline.as_ref() {
        Some(source) => {
            if source.project_id != input.project_id
                || source.chapter_number != input.chapter_number
            {
                return Err(ChapterGenerationLifecycleError::InvalidArgument(
                    "正文生成来源章纲与目标章节不一致".to_string(),
                ));
            }
            crate::project::formal_chapter_outline_id_if_unchanged(db, source)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
        }
        None => None,
    };
    let updated_chapter = match input.existing_chapter_id {
        Some(chapter_id) => {
            let status = chapter::Entity::find_by_id(chapter_id)
                .select_only()
                .column(chapter::Column::Status)
                .into_tuple::<String>()
                .one(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
                .ok_or_else(|| {
                    ChapterGenerationLifecycleError::NotFound(format!("章节不存在: {chapter_id}"))
                })?;
            if status != chapter::STATUS_GENERATING {
                return Err(ChapterGenerationLifecycleError::InvalidState {
                    current: status,
                    operation: "完成章节生成".to_string(),
                });
            }

            let mut active = chapter::ActiveModel {
                id: Set(chapter_id),
                ..Default::default()
            };
            assign_chapter_content(
                &mut active,
                input.content,
                input.word_count,
                STATUS_DRAFT,
                Some(input.summary),
                input.now,
            );
            active.title = Set(Some(input.title));
            active.outline_id = Set(outline_id);
            active
                .update(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
        }
        None => {
            let mut active = chapter::ActiveModel {
                project_id: Set(input.project_id),
                chapter_number: Set(input.chapter_number),
                title: Set(Some(input.title)),
                outline_id: Set(outline_id),
                created_at: Set(input.now),
                ..Default::default()
            };
            assign_chapter_content(
                &mut active,
                input.content,
                input.word_count,
                STATUS_DRAFT,
                Some(input.summary),
                input.now,
            );
            active
                .insert(db)
                .await
                .map_err(ChapterGenerationLifecycleError::database)?
        }
    };

    delete_generation_run_targets_for(db, "chapter_content", &[updated_chapter.id]).await?;
    delete_generation_run_targets_for(db, "chapter_analysis", &[updated_chapter.id]).await?;
    Ok(crate::project::chapter_record(updated_chapter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db_models::models::chapter::chapter;
    use a4n_db_models::models::task::task_queue;
    use chrono::Utc;
    use sea_orm::Set;
    use std::sync::Arc;

    async fn create_chapter(
        db: &impl crate::DbExecutor,
        project_id: i64,
        chapter_number: i32,
    ) -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::ActiveModel {
            project_id: Set(project_id),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第{chapter_number}章"))),
            content: Set(String::new()),
            word_count: Set(0),
            status: Set(chapter::STATUS_DRAFT.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("chapter must be inserted")
    }

    #[tokio::test]
    async fn only_one_chapter_per_project_can_enter_generation() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let user = crate::auth::create_user_summary(
            &db,
            "chapter_generation_lock",
            &format!(
                "chapter-generation-lock-{}@example.invalid",
                uuid::Uuid::new_v4()
            ),
            "test-password",
            "user",
        )
        .await
        .expect("user must be created");
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: user.id,
                title: format!("chapter generation lock {}", uuid::Uuid::new_v4()),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .expect("project must be created");
        create_chapter(&db, project.id, 1).await;
        create_chapter(&db, project.id, 2).await;
        crate::package::ensure_host_package_states(&db, &["ai-workflow".to_string()])
            .await
            .expect("workflow package state must be initialized");
        let handle = DbHandle::new(Arc::new(db.clone()));

        let first_task = reserve_chapter_generation_task(&db, user.id, project.id, 1).await;
        let second_task = reserve_chapter_generation_task(&db, user.id, project.id, 2).await;
        let first_queue_record = task_queue::Entity::find_by_id(first_task.clone())
            .one(&db)
            .await
            .expect("first task queue row must load")
            .expect("first task queue row must exist");
        let mut second_queue_record: task_queue::ActiveModel =
            task_queue::Entity::find_by_id(second_task.clone())
                .one(&db)
                .await
                .expect("second task queue row must load")
                .expect("second task queue row must exist")
                .into();
        second_queue_record.created_at =
            Set(first_queue_record.created_at + chrono::Duration::seconds(1));
        second_queue_record
            .update(&db)
            .await
            .expect("second task queue order must be deterministic");
        let second_error = begin_chapter_generation_record(
            &handle,
            project.id,
            2,
            &second_task,
            Utc::now().naive_utc(),
        )
        .await
        .expect_err("later queued chapter must wait for the earlier task");
        assert!(matches!(
            second_error,
            ChapterGenerationLifecycleError::InvalidState { current, .. }
                if current == "another_chapter_generation_task_is_ahead_in_queue"
        ));

        let first = begin_chapter_generation_record(
            &handle,
            project.id,
            1,
            &first_task,
            Utc::now().naive_utc(),
        )
        .await
        .expect("first chapter generation must start")
        .expect("chapter one must exist");
        let error = begin_chapter_generation_record(
            &handle,
            project.id,
            2,
            &second_task,
            Utc::now().naive_utc(),
        )
        .await
        .expect_err("another chapter in the project must not start concurrently");
        assert!(matches!(
            error,
            ChapterGenerationLifecycleError::InvalidState { operation, .. }
                if operation.contains("第1章正在生成")
        ));

        assert!(
            restore_chapter_generation_status_record(
                &handle,
                first.id,
                first.previous_status,
                Utc::now().naive_utc(),
            )
            .await
            .expect("first chapter status must be restored")
        );
    }

    async fn reserve_chapter_generation_task(
        db: &impl crate::DbExecutor,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
    ) -> String {
        crate::generation::reserve_generation_task(
            db,
            crate::generation::ReserveGenerationTaskInput {
                requested_by_user_id: user_id,
                start_request_id: uuid::Uuid::new_v4().to_string(),
                business_key_json: serde_json::json!({
                    "version": 1,
                    "task_kind": "chapter_generation",
                    "project_id": project_id,
                    "chapter_number": chapter_number,
                }),
                input_fingerprint_json: serde_json::json!({
                    "chapter_number": chapter_number,
                }),
                task_kind: a4n_queue::priority::TaskKind::LlmCall,
                priority: a4n_queue::priority::TaskPriority::High,
                delivery_mode: a4n_service_protocol::GenerationDeliveryMode::Stream,
                batch_policy: None,
                project_id: Some(project_id),
                root_project_id: None,
                source_snapshot_id: None,
                task_scope: Some("chapter_generation".to_string()),
                workflow_run_id: None,
                generation_run_id: None,
                target: Some(a4n_service_protocol::GenerationTargetRef {
                    kind: "chapter".to_string(),
                    id: format!("{project_id}:{chapter_number}"),
                    revision: None,
                }),
                root_task_id: None,
                parent_task_id: None,
                item_key: None,
                retry_of_task_id: None,
            },
        )
        .await
        .expect("chapter generation task must be reserved")
        .task
        .checkpoint
        .task_id
    }
}
