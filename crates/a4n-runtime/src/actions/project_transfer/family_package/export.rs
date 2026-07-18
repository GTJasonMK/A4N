mod media;
mod package;
mod selection;

use crate::state::AppState;

use media::package_zip_bytes;
use package::build_export_package;
use selection::{load_derived_project_export_selection, load_project_family_export_selection};

pub(super) async fn export_project_family_package_bytes(
    state: &AppState,
    root_project_id: i64,
    include_derived_projects: bool,
) -> Result<Vec<u8>, String> {
    let (root, derived_projects) =
        load_project_family_export_selection(state, root_project_id, include_derived_projects)
            .await?;
    let package = build_export_package(state, root, derived_projects, "project_family").await?;
    package_zip_bytes(state, &package).await
}

pub(super) async fn export_derived_project_package_bytes(
    state: &AppState,
    derived_project_id: i64,
) -> Result<Vec<u8>, String> {
    let (root, derived_projects) =
        load_derived_project_export_selection(state, derived_project_id).await?;
    let package = build_export_package(state, root, derived_projects, "derived_project").await?;
    package_zip_bytes(state, &package).await
}
