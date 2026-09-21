use super::SceneProfileDbError;
use super::counts::validate_scene_occurrence_count_ids;
use sea_orm::DbErr;
use std::error::Error;

#[test]
fn scene_occurrence_count_ids_reject_invalid_project_and_profile_ids() {
    assert!(matches!(
        validate_scene_occurrence_count_ids(0, &[]),
        Err(SceneProfileDbError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
    assert!(matches!(
        validate_scene_occurrence_count_ids(1, &[3, 0, 5]),
        Err(SceneProfileDbError::InvalidArgument(message))
            if message == "scene_profile_id 必须大于 0: 0"
    ));
    assert!(validate_scene_occurrence_count_ids(1, &[]).is_ok());
    assert!(validate_scene_occurrence_count_ids(1, &[3, 5]).is_ok());
}

#[test]
fn scene_profile_database_error_keeps_db_source() {
    let error = SceneProfileDbError::Database(DbErr::Custom("fixture failure".to_string()));
    assert!(error.source().is_some());
    assert!(error.to_string().contains("fixture failure"));
}
