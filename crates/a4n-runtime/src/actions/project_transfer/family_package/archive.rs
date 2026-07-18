mod lifecycle;
mod payload;
mod reader;
#[cfg(test)]
mod tests;

use a4n_core::models::project::novel_project;

use crate::state::AppState;

use lifecycle::import_project_family_package_parts;
use reader::read_project_family_package_bytes;

pub(super) async fn import_project_family_package_bytes(
    state: &AppState,
    user_id: i64,
    bytes: Vec<u8>,
) -> Result<Vec<novel_project::Model>, String> {
    if bytes.is_empty() {
        return Err("项目族导入包不能为空".to_string());
    }
    let (package, media_payload) = read_project_family_package_bytes(state, bytes).await?;
    import_project_family_package_parts(state, user_id, package, media_payload).await
}
