use super::*;
use a4n_chapter_scene_worker_contract::{
    CHAPTER_SCENE_DELETE_OPERATION, CHAPTER_SCENE_REORDER_OPERATION, CHAPTER_SCENE_UPDATE_OPERATION,
};

impl WorkerState {
    pub(super) async fn update(
        &self,
        request: WorkerEnvelope<UpdateSceneInput>,
    ) -> Result<ChapterSceneSummary, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_UPDATE_OPERATION)?;
        let actor_user_id = user_id(&request)?;
        let input = request.payload;
        let scene =
            a4n_db::project::chapter_scene_by_id(self.database.connection(), input.scene_id)
                .await
                .map_err(record_error)?
                .ok_or_else(|| {
                    WorkerFailure::not_found(format!("场景不存在: scene_id={}", input.scene_id))
                })?;
        let chapter = self
            .authorized_chapter(scene.chapter_id, actor_user_id)
            .await?;
        let content = input
            .content
            .map(|value| {
                let value = value.trim().to_string();
                if value.is_empty() {
                    Err(WorkerFailure::invalid_request("场景正文不能为空"))
                } else {
                    Ok(value)
                }
            })
            .transpose()?;
        let tx = self
            .database
            .begin()
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let result = async {
            a4n_db::generation::require_project_lifecycle_active(
                &tx,
                actor_user_id,
                chapter.project_id,
            )
            .await
            .map_err(lifecycle_gate_error)?;
            let scene = a4n_db::project::update_chapter_scene_record(
                &tx,
                a4n_db::project::ProjectChapterSceneUpdateInput {
                    scene,
                    content,
                    title: input.title.map(|value| Some(trim_optional(value))),
                    summary: input.summary.map(|value| Some(trim_optional(value))),
                    scene_type: input
                        .scene_type
                        .map(|value| required_text(value, "场景类型"))
                        .transpose()?,
                    location: input.location.map(|value| Some(trim_optional(value))),
                    timeline_position: input
                        .timeline_position
                        .map(|value| Some(trim_optional(value))),
                    updated_at: Utc::now().naive_utc(),
                },
            )
            .await
            .map_err(record_error)?;
            Ok(scene_summary(scene))
        }
        .await;
        finish_mutation_transaction(tx, result).await
    }

    pub(super) async fn delete(
        &self,
        request: WorkerEnvelope<DeleteSceneInput>,
    ) -> Result<bool, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_DELETE_OPERATION)?;
        let scene = a4n_db::project::chapter_scene_by_id(
            self.database.connection(),
            request.payload.scene_id,
        )
        .await
        .map_err(record_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("场景不存在: scene_id={}", request.payload.scene_id))
        })?;
        let actor_user_id = user_id(&request)?;
        let chapter = self
            .authorized_chapter(scene.chapter_id, actor_user_id)
            .await?;
        let tx = self
            .database
            .begin()
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let result = async {
            a4n_db::generation::require_project_lifecycle_active(
                &tx,
                actor_user_id,
                chapter.project_id,
            )
            .await
            .map_err(lifecycle_gate_error)?;
            let rows = a4n_db::project::delete_chapter_scene_record(&tx, scene.id)
                .await
                .map_err(record_error)?;
            if rows == 0 {
                return Err(WorkerFailure::not_found(format!(
                    "场景不存在: scene_id={}",
                    scene.id
                )));
            }
            Ok(true)
        }
        .await;
        finish_mutation_transaction(tx, result).await
    }

    pub(super) async fn reorder(
        &self,
        request: WorkerEnvelope<ReorderScenesInput>,
    ) -> Result<Vec<ChapterSceneSummary>, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_REORDER_OPERATION)?;
        if request.payload.order.is_empty() {
            return Err(WorkerFailure::invalid_request("order 不能为空"));
        }
        let project_id = a4n_db::project::chapter_scene_reorder_project_id(
            self.database.connection(),
            &request.payload.order,
        )
        .await
        .map_err(write_error)?;
        let actor_user_id = user_id(&request)?;
        verify_project_editor(&self.database, project_id, actor_user_id).await?;
        let tx = self
            .database
            .begin()
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let result = async {
            a4n_db::generation::require_project_lifecycle_active(&tx, actor_user_id, project_id)
                .await
                .map_err(lifecycle_gate_error)?;
            let scenes = a4n_db::project::reorder_chapter_scene_records_in_transaction(
                &tx,
                &request.payload.order,
            )
            .await
            .map_err(write_error)?;
            Ok(scenes.into_iter().map(scene_summary).collect())
        }
        .await;
        finish_mutation_transaction(tx, result).await
    }
}

async fn finish_mutation_transaction<T>(
    tx: a4n_db::DbTransaction,
    result: Result<T, WorkerFailure>,
) -> Result<T, WorkerFailure> {
    match result {
        Ok(value) => {
            tx.commit()
                .await
                .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
            Ok(value)
        }
        Err(error) => match tx.rollback().await {
            Ok(()) => Err(error),
            Err(rollback_error) => Err(WorkerFailure::unavailable(format!(
                "{}；事务回滚失败：{rollback_error}",
                error.message
            ))),
        },
    }
}

fn lifecycle_gate_error(error: a4n_db::generation::GenerationLifecycleGateError) -> WorkerFailure {
    use a4n_db::generation::GenerationLifecycleGateError as Error;
    match error {
        Error::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        Error::NotFound(message) => WorkerFailure::not_found(message),
        Error::Blocked(message) => WorkerFailure::conflict(message),
        Error::Corrupt(message) => WorkerFailure::unavailable(message),
        Error::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}
