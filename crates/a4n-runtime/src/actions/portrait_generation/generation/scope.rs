use super::*;

pub async fn generate_portrait_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GeneratePortraitInput,
) -> Result<PortraitTaskScope, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    Ok(PortraitTaskScope {
        project_id: input.project_id,
        total: 6,
    })
}

pub async fn regenerate_portrait_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegeneratePortraitInput,
) -> Result<PortraitTaskScope, String> {
    let project_id = portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    Ok(PortraitTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn generate_age_variant_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateAgeVariantInput,
) -> Result<PortraitTaskScope, String> {
    let project_id = portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    Ok(PortraitTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn generate_missing_portraits_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateMissingPortraitsInput,
) -> Result<PortraitTaskScope, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    Ok(PortraitTaskScope {
        project_id: input.project_id,
        total: 1,
    })
}
