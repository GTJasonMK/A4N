use super::super::super::{
    AppState, OBJECT_PROFILE_IMAGE_SPEC, ObjectProfileImageInput, ProfileImageMaterial,
    SCENE_PROFILE_IMAGE_SPEC, SceneProfileImageInput, access, object_profile_content,
    object_profile_image_prompt, scene_profile_content, scene_profile_image_prompt,
};
use super::metadata::{profile_material_style_metadata, profile_source_material};

macro_rules! profile_material_builder {
    (
        $fn_name:ident,
        $input_ty:ty,
        $id_field:ident,
        $orch_get:ident,
        $id_error:literal,
        $prompt_fn:ident,
        $content_fn:ident,
        $spec:expr,
        [$($source_field:ident),+ $(,)?] $(,)?
    ) => {
        pub(in crate::actions::image_generation) async fn $fn_name(
            state: &AppState,
            user_id: i64,
            input: &$input_ty,
        ) -> Result<ProfileImageMaterial, String> {
            if input.$id_field <= 0 {
                return Err($id_error.to_string());
            }
            let profile = state
                .orchestrator
                .$orch_get(input.$id_field)
                .await
                .map_err(|error| error.to_string())?;
            access::verify_project_editor_string(state, user_id, profile.project_id).await?;
            let prompt = $prompt_fn(&profile, input.style.as_deref());
            let source_material =
                profile_source_material(&[$(profile.$source_field.as_str()),+], &prompt);
            let profile_content = $content_fn(&profile);
            Ok(ProfileImageMaterial {
                spec: $spec,
                profile_id: profile.id,
                project_id: profile.project_id,
                name: profile.name,
                prompt,
                profile_content,
                source_material,
                style_metadata: profile_material_style_metadata(
                    input.style_template_id.clone(),
                    input.style_template_name.clone(),
                    input.style_template_version.clone(),
                    input.custom_style.clone(),
                ),
            })
        }
    };
}

profile_material_builder!(
    build_object_profile_material,
    ObjectProfileImageInput,
    object_profile_id,
    object_profile_by_id,
    "object_profile_id 必须大于 0",
    object_profile_image_prompt,
    object_profile_content,
    OBJECT_PROFILE_IMAGE_SPEC,
    [
        name,
        visual_description,
        material_and_state,
        story_usage,
        visual_usage,
        generation_usage,
        image_prompt_seed,
    ],
);

profile_material_builder!(
    build_scene_profile_material,
    SceneProfileImageInput,
    scene_profile_id,
    scene_profile_by_id,
    "scene_profile_id 必须大于 0",
    scene_profile_image_prompt,
    scene_profile_content,
    SCENE_PROFILE_IMAGE_SPEC,
    [
        name,
        base_description,
        visual_description,
        spatial_layout,
        atmosphere,
        image_prompt_seed,
    ],
);
