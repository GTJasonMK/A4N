use super::*;

impl WorkerState {
    pub(super) async fn create(
        &self,
        request: WorkerEnvelope<CreateObjectProfileInput>,
    ) -> Result<ObjectProfileMutationResponse, WorkerFailure> {
        validate_request(&request, OBJECT_PROFILE_CREATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let project_id = request.payload.project_id;
        if request.project_id != Some(project_id) || project_id <= 0 {
            return Err(WorkerFailure::invalid_request(
                "worker scope project_id 与 payload 不一致或无效",
            ));
        }
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(collaboration_error)?;
        let record = normalize_object_profile_record_input(request.payload.profile)
            .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        let profile = a4n_db::asset::create_object_profile_read_record(
            self.database.connection(),
            project_id,
            record,
        )
        .await
        .map_err(asset_error)?;
        Ok(ObjectProfileMutationResponse {
            result: ObjectProfileMutationResult {
                profile: object_profile_summary(&profile, 0),
                warnings: Vec::new(),
            },
            downstream_image_ids: Vec::new(),
        })
    }

    pub(super) async fn update(
        &self,
        request: WorkerEnvelope<UpdateObjectProfileInput>,
    ) -> Result<ObjectProfileMutationResponse, WorkerFailure> {
        validate_request(&request, OBJECT_PROFILE_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let existing = a4n_db::asset::object_profile_record_by_id(
            self.database.connection(),
            request.payload.object_profile_id,
        )
        .await
        .map_err(asset_error)?;
        ensure_scope_project(&request, existing.project_id)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            existing.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(collaboration_error)?;
        let image_ids = self
            .object_profile_image_ids(existing.project_id, existing.id)
            .await?;
        let image_count = i64::try_from(image_ids.len())
            .map_err(|_| WorkerFailure::conflict("物体档案素材数量超过 i64 上限"))?;
        let downstream_image_ids = resolve_downstream_image_ids(
            image_ids,
            request.payload.downstream_strategy,
            &format!("物体档案 #{} ", existing.id),
            "修改",
        )
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        let record = normalize_object_profile_record_input(request.payload.profile)
            .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        let profile = a4n_db::asset::update_object_profile_read_record(
            self.database.connection(),
            existing.id,
            &request.payload.expected_updated_at,
            record,
        )
        .await
        .map_err(asset_error)?;
        Ok(ObjectProfileMutationResponse {
            result: ObjectProfileMutationResult {
                profile: object_profile_summary(&profile, image_count),
                warnings: Vec::new(),
            },
            downstream_image_ids,
        })
    }

    pub(super) async fn delete(
        &self,
        request: WorkerEnvelope<DeleteObjectProfileInput>,
    ) -> Result<ObjectProfileDeleteResponse, WorkerFailure> {
        validate_request(&request, OBJECT_PROFILE_DELETE_OPERATION)?;
        let user_id = user_id(&request)?;
        let existing = a4n_db::asset::object_profile_record_by_id(
            self.database.connection(),
            request.payload.object_profile_id,
        )
        .await
        .map_err(asset_error)?;
        ensure_scope_project(&request, existing.project_id)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            existing.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(collaboration_error)?;
        let downstream_image_ids = resolve_downstream_image_ids(
            self.object_profile_image_ids(existing.project_id, existing.id)
                .await?,
            request.payload.downstream_strategy,
            &format!("物体档案 #{} ", existing.id),
            "删除",
        )
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        a4n_db::asset::delete_object_profile_record(self.database.connection(), existing.id)
            .await
            .map_err(asset_error)?;
        Ok(ObjectProfileDeleteResponse {
            result: ObjectProfileDeleteResult {
                warnings: Vec::new(),
            },
            downstream_image_ids,
        })
    }

    async fn object_profile_image_ids(
        &self,
        project_id: i64,
        profile_id: i64,
    ) -> Result<Vec<i64>, WorkerFailure> {
        let rows = a4n_db::profile_material::profile_material_image_identity_rows_by_type(
            self.database.connection(),
            project_id,
            a4n_domain::image::IMAGE_TYPE_OBJECT_MATERIAL,
        )
        .await
        .map_err(profile_material_error)?;
        Ok(rows
            .into_iter()
            .filter(|row| {
                a4n_db::profile_material::profile_material_identity(
                    &row.image_type,
                    row.metadata_json.as_ref(),
                ) == Some((a4n_db::profile_material::PROFILE_KIND_OBJECT, profile_id))
            })
            .map(|row| row.id)
            .collect())
    }
}
