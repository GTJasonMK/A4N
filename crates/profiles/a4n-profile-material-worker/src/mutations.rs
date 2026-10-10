use super::*;

impl WorkerState {
    pub(super) async fn set_primary(
        &self,
        request: WorkerEnvelope<SetProfilePrimaryMaterialInput>,
    ) -> Result<ProfileMaterialSetPrimaryResponse, WorkerFailure> {
        validate_request(&request, PROFILE_MATERIAL_SET_PRIMARY_OPERATION)?;
        request.require_project_scope(request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        let input = request.payload;
        verify_permission(
            &self.database,
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.ensure_profile(input.project_id, input.profile_kind, input.profile_id)
            .await?;
        if let Some(image_id) = input.image_id {
            let image = a4n_db::video::read_models::generated_image_by_id(
                self.database.connection(),
                image_id,
            )
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| WorkerFailure::not_found("图片不存在"))?;
            if image.project_id != input.project_id {
                return Err(WorkerFailure::invalid_request(format!(
                    "图片 #{image_id} 不属于项目 {}",
                    input.project_id
                )));
            }
            if profile_material_identity(&image.image_type, image.metadata_json.as_ref())
                != Some((input.profile_kind.as_str(), input.profile_id))
            {
                return Err(WorkerFailure::invalid_request(format!(
                    "图片 #{image_id} 不是 {} 档案 #{} 的素材图",
                    input.profile_kind.as_str(),
                    input.profile_id
                )));
            }
            a4n_db::profile_material::upsert_profile_primary_material(
                self.database.connection(),
                input.project_id,
                input.profile_kind.as_str(),
                input.profile_id,
                image_id,
            )
            .await
            .map_err(profile_material_error)?;
        } else {
            a4n_db::profile_material::delete_profile_primary_material(
                self.database.connection(),
                input.project_id,
                input.profile_kind.as_str(),
                input.profile_id,
            )
            .await
            .map_err(profile_material_error)?;
        }
        let group = self
            .profile_material_group(input.project_id, input.profile_kind, input.profile_id)
            .await?;
        Ok(ProfileMaterialSetPrimaryResponse { group })
    }

    async fn ensure_profile(
        &self,
        project_id: i64,
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<(), WorkerFailure> {
        let profile_project_id = match kind {
            ProfileMaterialKind::Scene => a4n_db::scene_profile::scene_profile_project_id(
                self.database.connection(),
                profile_id,
            )
            .await
            .map_err(scene_profile_error)?,
            ProfileMaterialKind::Object => {
                a4n_db::asset::object_profile_project_id(self.database.connection(), profile_id)
                    .await
                    .map_err(asset_error)?
            }
        };
        if profile_project_id != project_id {
            let label = match kind {
                ProfileMaterialKind::Scene => "场景档案",
                ProfileMaterialKind::Object => "物体档案",
            };
            return Err(WorkerFailure::invalid_request(format!(
                "{label} #{profile_id} 不属于项目 {project_id}"
            )));
        }
        Ok(())
    }

    async fn profile_material_group(
        &self,
        project_id: i64,
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<ProfileMaterialGroup, WorkerFailure> {
        let image_types = [image_type_for_profile_material_kind(kind)];
        let (source, rows) = tokio::join!(
            async {
                match kind {
                    ProfileMaterialKind::Scene => a4n_db::scene_profile::scene_profile_by_id(
                        self.database.connection(),
                        profile_id,
                    )
                    .await
                    .map(|profile| {
                        scene_source_from_record(
                            a4n_db::profile_material::scene_profile_source_record(&profile),
                        )
                    })
                    .map_err(scene_profile_error),
                    ProfileMaterialKind::Object => {
                        a4n_db::asset::object_profile_by_id(self.database.connection(), profile_id)
                            .await
                            .map(|profile| {
                                object_source_from_record(
                                    a4n_db::profile_material::object_profile_source_record(
                                        &profile,
                                    ),
                                )
                            })
                            .map_err(asset_error)
                    }
                }
            },
            async {
                a4n_db::profile_material::profile_material_list_rows(
                    self.database.connection(),
                    project_id,
                    &image_types,
                )
                .await
                .map_err(profile_material_error)
            },
        );
        let source = source?;
        if source.profile.project_id != project_id {
            return Err(WorkerFailure::invalid_request(format!(
                "档案 #{} 不属于项目 {project_id}",
                profile_id
            )));
        }
        let rows = rows?;
        profile_material_group_projections(vec![source], rows.images, rows.primary_rows, 32)
            .into_iter()
            .next()
            .map(|projection| projection.group)
            .ok_or_else(|| WorkerFailure::conflict("主素材已更新，但无法重新读取档案素材"))
    }
}
