use super::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ObjectProfileDeleteResult,
    ObjectProfileMutationResult, UpdateObjectProfileInput,
};
use crate::{
    AppState,
    actions::dependency_conflict::{
        delete_downstream_images_if_requested, downstream_cleanup_warnings,
    },
};

pub async fn create_object_profile(
    state: &AppState,
    user_id: i64,
    input: CreateObjectProfileInput,
) -> Result<ObjectProfileMutationResult, String> {
    let outcome = state
        .object_profile_mutation_application()
        .create(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut result = outcome.result;
    result.warnings.extend(downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, outcome.downstream_image_ids).await,
    ));
    Ok(result)
}

pub async fn update_object_profile(
    state: &AppState,
    user_id: i64,
    input: UpdateObjectProfileInput,
) -> Result<ObjectProfileMutationResult, String> {
    let outcome = state
        .object_profile_mutation_application()
        .update(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut result = outcome.result;
    result.warnings.extend(downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, outcome.downstream_image_ids).await,
    ));
    Ok(result)
}

pub async fn delete_object_profile(
    state: &AppState,
    user_id: i64,
    input: DeleteObjectProfileInput,
) -> Result<ObjectProfileDeleteResult, String> {
    let outcome = state
        .object_profile_mutation_application()
        .delete(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)?;
    let mut result = outcome.result;
    result.warnings.extend(downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, outcome.downstream_image_ids).await,
    ));
    Ok(result)
}
