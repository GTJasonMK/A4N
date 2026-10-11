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
        let actor_user_id = request.principal.require_user_id()?;
        let input = request.payload;
        let scene =
            a4n_db::project::chapter_scene_by_id(self.database.connection(), input.scene_id)
                .await
                .map_err(record_error)?
                .ok_or_else(|| {
                    WorkerFailure::not_found(format!("场景不存在: scene_id={}", input.scene_id))
                })?;
        self.authorized_chapter(scene.chapter_id, actor_user_id)
            .await?;
        let content = input
            .content
            .map(|value| {
                WorkerFailure::require_non_blank(&value, "场景正文不能为空").map(str::to_owned)
            })
            .transpose()?;
        let scene = a4n_db::project::update_chapter_scene_record(
            self.database.connection(),
            a4n_db::project::ProjectChapterSceneUpdateInput {
                scene,
                content,
                title: input.title.map(|value| Some(trim_optional(value))),
                summary: input.summary.map(|value| Some(trim_optional(value))),
                scene_type: input
                    .scene_type
                    .map(|value| {
                        WorkerFailure::require_non_blank(&value, "场景类型不能为空")
                            .map(str::to_owned)
                    })
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
        self.authorized_chapter(scene.chapter_id, request.principal.require_user_id()?)
            .await?;
        let rows =
            a4n_db::project::delete_chapter_scene_record(self.database.connection(), scene.id)
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
        verify_project_editor(
            &self.database,
            project_id,
            request.principal.require_user_id()?,
        )
        .await?;
        let scenes = a4n_db::project::reorder_chapter_scene_records(
            self.database.connection(),
            &request.payload.order,
        )
        .await
        .map_err(write_error)?;
        Ok(scenes.into_iter().map(scene_summary).collect())
    }
}
