use super::contracts::ExportTaskScope;
use crate::{AppState, actions::access::verify_project_editor_string};

macro_rules! export_project_scope {
    ($name:ident, $total:expr) => {
        pub async fn $name(
            state: &AppState,
            user_id: i64,
            project_id: i64,
        ) -> Result<ExportTaskScope, String> {
            export_project_task_scope(state, user_id, project_id, $total).await
        }
    };
}

export_project_scope!(export_epub_task_scope, 4);
export_project_scope!(export_txt_task_scope, 2);
export_project_scope!(export_markdown_task_scope, 2);
export_project_scope!(export_pdf_task_scope, 5);
export_project_scope!(export_outline_json_task_scope, 2);
export_project_scope!(export_characters_json_task_scope, 2);

pub async fn export_batch_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportTaskScope, String> {
    export_project_task_scope(state, user_id, project_id, 4).await
}

async fn export_project_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    total: u32,
) -> Result<ExportTaskScope, String> {
    verify_project_editor_string(state, user_id, project_id).await?;
    Ok(ExportTaskScope { project_id, total })
}
