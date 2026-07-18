use super::persist_galgame_script_record;
use crate::galgame::{GalgameScriptRecordError, PersistGalgameScriptRecordInput};
use sea_orm::DatabaseConnection;
use serde_json::json;

#[tokio::test]
async fn persist_script_rejects_empty_scenes_before_database_access() {
    let input = PersistGalgameScriptRecordInput {
        project_id: 1,
        root_project_id: 1,
        source_snapshot_id: 2,
        title: "测试 Galgame".to_string(),
        style_config_json: json!({}),
        status: "draft".to_string(),
        scenes: Vec::new(),
    };

    assert!(matches!(
        persist_galgame_script_record(&DatabaseConnection::Disconnected, input).await,
        Err(GalgameScriptRecordError::InvalidArgument(message))
            if message == "Galgame persist_script 至少需要一个场"
    ));
}
