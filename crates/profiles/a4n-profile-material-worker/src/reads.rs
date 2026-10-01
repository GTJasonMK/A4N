use super::*;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ListProfileMaterialsInput>,
    ) -> Result<Vec<ProfileMaterialGroup>, WorkerFailure> {
        validate_request(&request, PROFILE_MATERIAL_LIST_OPERATION)?;
        validate_input_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.image_limit_per_profile == 0 || input.image_limit_per_profile > MAX_IMAGE_LIMIT {
            return Err(WorkerFailure::invalid_request(
                "image_limit_per_profile 必须在 1 到 32 之间",
            ));
        }
        verify_permission(
            &self.database,
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let kinds = requested_profile_material_kinds(input.kind);
        let image_types = kinds
            .iter()
            .map(|kind| image_type_for_profile_material_kind(*kind))
            .collect::<Vec<_>>();
        let include_scene = kinds.contains(&ProfileMaterialKind::Scene);
        let include_object = kinds.contains(&ProfileMaterialKind::Object);
        let (scene_profiles, object_profiles, rows) = tokio::join!(
            async {
                if include_scene {
                    a4n_db::scene_profile::scene_profiles_by_project(
                        self.database.connection(),
                        input.project_id,
                    )
                    .await
                    .map_err(scene_profile_error)
                } else {
                    Ok(Vec::new())
                }
            },
            async {
                if include_object {
                    a4n_db::asset::object_profiles_by_project(
                        self.database.connection(),
                        input.project_id,
                    )
                    .await
                    .map_err(asset_error)
                } else {
                    Ok(Vec::new())
                }
            },
            async {
                a4n_db::profile_material::profile_material_list_rows(
                    self.database.connection(),
                    input.project_id,
                    &image_types,
                )
                .await
                .map_err(profile_material_error)
            },
        );
        let mut sources = scene_profiles?
            .into_iter()
            .map(|profile| a4n_db::profile_material::scene_profile_source_record(&profile))
            .map(scene_source_from_record)
            .collect::<Vec<_>>();
        sources.extend(
            object_profiles?
                .into_iter()
                .map(|profile| a4n_db::profile_material::object_profile_source_record(&profile))
                .map(object_source_from_record),
        );
        let rows = rows?;
        let mut groups = profile_material_group_projections(
            sources,
            rows.images,
            rows.primary_rows,
            input.image_limit_per_profile,
        );
        apply_profile_material_query(&mut groups, &input);
        Ok(groups
            .into_iter()
            .map(|projection| projection.group)
            .collect())
    }
}
