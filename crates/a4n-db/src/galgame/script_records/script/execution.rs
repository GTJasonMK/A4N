use super::deletion::delete_galgame_script_records_for_project;
use super::persistence::persist_galgame_script_record;
use crate::galgame::{
    GalgameScriptRecordError, PersistGalgameScriptRecordInput,
    delete_galgame_target_generation_media_graph_record,
};
use a4n_db_models::models::galgame::galgame_script;
use sea_orm::{DatabaseConnection, TransactionTrait};

pub async fn replace_galgame_script_for_project_record(
    db: &DatabaseConnection,
    project_id: i64,
    input: PersistGalgameScriptRecordInput,
) -> Result<galgame_script::Model, GalgameScriptRecordError> {
    let tx = db.begin().await?;
    delete_galgame_target_generation_media_graph_record(&tx, project_id).await?;
    delete_galgame_script_records_for_project(&tx, project_id).await?;
    let script = persist_galgame_script_record(&tx, input).await?;
    tx.commit().await?;
    Ok(script)
}
