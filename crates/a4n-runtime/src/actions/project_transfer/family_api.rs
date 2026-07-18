use super::{
    contracts::{
        ExportDerivedProjectDataInput, ExportProjectFamilyDataInput, ExportProjectPackageData,
        ImportProjectFamilyUploadInput,
    },
    family_package::{
        export_derived_project_package_bytes, export_project_family_package_bytes,
        import_project_family_package_bytes,
    },
    package_files::{
        decode_uploaded_project_file, export_package_data, project_record_by_id,
        validate_uploaded_project_file_name,
    },
};
use crate::{actions::access, state::AppState};
use a4n_core::models::project::novel_project;

pub async fn export_project_family_data(
    state: &AppState,
    user_id: i64,
    input: ExportProjectFamilyDataInput,
) -> Result<ExportProjectPackageData, String> {
    access::verify_project_owner_string(state, user_id, input.root_project_id).await?;
    let project = project_record_by_id(state, input.root_project_id).await?;
    let bytes = export_project_family_package_bytes(
        state,
        input.root_project_id,
        input.include_derived_projects,
    )
    .await?;
    Ok(export_package_data(&project, bytes))
}

pub async fn export_derived_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportDerivedProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    access::verify_project_owner_string(state, user_id, input.derived_project_id).await?;
    let project = project_record_by_id(state, input.derived_project_id).await?;
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("派生项目缺少 root_project_id: {}", project.id))?;
    access::verify_project_owner_string(state, user_id, root_project_id).await?;
    let bytes = export_derived_project_package_bytes(state, input.derived_project_id).await?;
    Ok(export_package_data(&project, bytes))
}

pub async fn import_project_family_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectFamilyUploadInput,
) -> Result<Vec<novel_project::Model>, String> {
    validate_uploaded_project_file_name(&input.file_name, "zip", "项目族包")?;
    let bytes = decode_uploaded_project_file(&input.data_base64, "项目族包")?;
    import_project_family_package_bytes(state, user_id, bytes).await
}
