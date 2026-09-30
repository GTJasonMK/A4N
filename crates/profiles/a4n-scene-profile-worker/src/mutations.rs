use super::*;

impl WorkerState {
    pub(super) async fn sync_records(
        &self,
        request: WorkerEnvelope<SceneProfileSyncRecordsInput>,
    ) -> Result<SceneSyncRecordResult, WorkerFailure> {
        validate_request(&request, SCENE_PROFILE_SYNC_RECORDS_OPERATION)?;
        let project_id = request.payload.project_id;
        validate_required_project_scope(request.project_id, project_id)?;
        let user_id = user_id(&request)?;
        self.verify_editor(project_id, user_id).await?;
        let SceneProfileSyncRecordsInput {
            project_id,
            chapter_id,
            chapter_number,
            source,
            records,
            profile_evidence,
            generation_run,
        } = request.payload;
        a4n_db::scene_profile::replace_chapter_scene_sync_with_generation_run_records(
            self.database.connection(),
            project_id,
            chapter_id,
            chapter_number,
            source,
            records,
            profile_evidence,
            generation_run,
        )
        .await
        .map_err(scene_profile_error)
    }

    pub(super) async fn create(
        &self,
        request: WorkerEnvelope<CreateSceneProfileInput>,
    ) -> Result<SceneProfileMutationWorkerResult, WorkerFailure> {
        validate_request(&request, SCENE_PROFILE_CREATE_OPERATION)?;
        let project_id = request.payload.project_id;
        validate_project_scope(&request, project_id)?;
        let user_id = user_id(&request)?;
        self.verify_editor(project_id, user_id).await?;
        let record = a4n_domain::scene_profile::normalize_scene_profile_record_input(
            request.payload.profile,
        )
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        let profile = a4n_db::scene_profile::create_scene_profile_record_checked(
            self.database.connection(),
            project_id,
            record,
        )
        .await
        .map_err(scene_profile_error)?;
        Ok(SceneProfileMutationWorkerResult {
            result: SceneProfileMutationResult {
                profile: scene_profile_summary(&profile, 0, 0),
                warnings: Vec::new(),
            },
            downstream_image_ids: Vec::new(),
        })
    }

    pub(super) async fn update(
        &self,
        request: WorkerEnvelope<UpdateSceneProfileInput>,
    ) -> Result<SceneProfileMutationWorkerResult, WorkerFailure> {
        validate_request(&request, SCENE_PROFILE_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let existing = a4n_db::scene_profile::scene_profile_by_id(
            self.database.connection(),
            request.payload.scene_profile_id,
        )
        .await
        .map_err(scene_profile_error)?;
        validate_project_scope(&request, existing.project_id)?;
        self.verify_editor(existing.project_id, user_id).await?;
        let image_ids = self
            .scene_image_ids(existing.project_id, existing.id)
            .await?;
        let image_count = i64::try_from(image_ids.len())
            .map_err(|_| WorkerFailure::conflict("场景档案素材数量超过 i64 上限"))?;
        let occurrence_count = a4n_db::scene_profile::scene_occurrence_count(
            self.database.connection(),
            existing.project_id,
            existing.id,
        )
        .await
        .map_err(scene_profile_error)?;
        let downstream_image_ids = resolve_downstream_image_ids(
            image_ids,
            request.payload.downstream_strategy,
            &format!("场景档案 #{} ", existing.id),
            "修改",
        )
        .map_err(WorkerFailure::invalid_request)?;
        let record = a4n_domain::scene_profile::normalize_scene_profile_record_input(
            request.payload.profile,
        )
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        let profile = a4n_db::scene_profile::update_scene_profile_record_checked(
            self.database.connection(),
            existing.id,
            record,
        )
        .await
        .map_err(scene_profile_error)?;
        Ok(SceneProfileMutationWorkerResult {
            result: SceneProfileMutationResult {
                profile: scene_profile_summary(&profile, occurrence_count, image_count),
                warnings: Vec::new(),
            },
            downstream_image_ids,
        })
    }

    pub(super) async fn delete(
        &self,
        request: WorkerEnvelope<DeleteSceneProfileInput>,
    ) -> Result<SceneProfileDeleteWorkerResult, WorkerFailure> {
        validate_request(&request, SCENE_PROFILE_DELETE_OPERATION)?;
        let user_id = user_id(&request)?;
        let existing = a4n_db::scene_profile::scene_profile_by_id(
            self.database.connection(),
            request.payload.scene_profile_id,
        )
        .await
        .map_err(scene_profile_error)?;
        validate_project_scope(&request, existing.project_id)?;
        self.verify_editor(existing.project_id, user_id).await?;
        let downstream_image_ids = resolve_downstream_image_ids(
            self.scene_image_ids(existing.project_id, existing.id)
                .await?,
            request.payload.downstream_strategy,
            &format!("场景档案 #{} ", existing.id),
            "删除",
        )
        .map_err(WorkerFailure::invalid_request)?;
        a4n_db::scene_profile::delete_scene_profile_records(
            self.database.connection(),
            existing.id,
        )
        .await
        .map_err(scene_profile_error)?;
        Ok(SceneProfileDeleteWorkerResult {
            result: SceneProfileDeleteResult {
                warnings: Vec::new(),
            },
            project_id: existing.project_id,
            downstream_image_ids,
        })
    }

    async fn verify_editor(&self, project_id: i64, user_id: i64) -> Result<(), WorkerFailure> {
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(collaboration_error)?;
        Ok(())
    }

    async fn scene_image_ids(
        &self,
        project_id: i64,
        profile_id: i64,
    ) -> Result<Vec<i64>, WorkerFailure> {
        a4n_db::profile_material::profile_material_image_ids_by_profile(
            self.database.connection(),
            project_id,
            IMAGE_TYPE_SCENE_MATERIAL,
            profile_id,
        )
        .await
        .map_err(profile_material_error)
    }
}
