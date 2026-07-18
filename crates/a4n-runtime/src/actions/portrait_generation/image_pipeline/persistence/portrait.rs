use a4n_core::models::character::character_portrait;
use a4n_orchestrator::character::CharacterPortraitCreateInput;

use crate::AppState;
use crate::file_utils::remove_created_file_on_error;

use super::super::contracts::{
    GeneratedPortraitImage, InsertPortraitInput, PersistGeneratedPortraitInput,
    PortraitImageMirror, PortraitImageProviderTrace,
};
use super::file::persist_portrait_file;
use super::mirror::mirror_or_rollback;

pub(in crate::actions::portrait_generation) async fn persist_generated_portrait(
    state: &AppState,
    generated: &GeneratedPortraitImage,
    input: PersistGeneratedPortraitInput<'_>,
) -> Result<character_portrait::Model, String> {
    let (path, path_str, file_size) = persist_portrait_file(
        &state.config.data_dir,
        input.project_id,
        input.profile_id,
        &generated.bytes,
        &generated.format,
        input.path_context,
    )?;
    let portrait = remove_created_file_on_error(&path, "立绘图片", async {
        insert_portrait(
            state,
            InsertPortraitInput {
                profile_id: input.profile_id,
                project_id: input.project_id,
                portrait_type: input.portrait_type,
                image_path: &path_str,
                prompt: input.prompt,
                expression: input.expression,
                pose: input.pose,
                is_primary: input.is_primary,
            },
        )
        .await
    })
    .await?;
    mirror_or_rollback(
        state,
        &portrait,
        &path_str,
        PortraitImageMirror {
            workflow_run_id: input.workflow_run_id,
            project_id: input.project_id,
            profile_id: input.profile_id,
            portrait_id: portrait.id,
            character_name: input.character_name,
            portrait_type: input.portrait_type,
            image_path: &path_str,
            prompt: input.prompt,
            model: &generated.model,
            width: generated.width,
            height: generated.height,
            file_size,
            expression: input.expression,
            pose: input.pose,
            variant_kind: input.variant_kind,
            auto_generated: input.auto_generated,
            reference_image_ids: input.reference_image_ids,
            provider_trace: Some(PortraitImageProviderTrace {
                provider: &generated.provider,
                request_json: &generated.request_json,
                output_format: &generated.format,
            }),
        },
    )
    .await?;
    Ok(portrait)
}

async fn insert_portrait(
    state: &AppState,
    input: InsertPortraitInput<'_>,
) -> Result<character_portrait::Model, String> {
    state
        .orchestrator
        .create_character_portrait_record(CharacterPortraitCreateInput {
            profile_id: input.profile_id,
            project_id: input.project_id,
            portrait_type: input.portrait_type.to_string(),
            image_path: input.image_path.to_string(),
            prompt: input.prompt.to_string(),
            expression: input.expression.map(str::to_string),
            pose: input.pose.map(str::to_string),
            is_primary: input.is_primary,
        })
        .await
        .map_err(|err| err.to_string())
}
