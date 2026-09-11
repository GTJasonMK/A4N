use a4n_api_contract::image::GeneratedImageSummary;
use a4n_orchestrator::video::VideoRecordApplicationPort;
use serde_json::json;

pub(super) async fn validated_reference_images_for_ids(
    video: &dyn VideoRecordApplicationPort,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<Vec<GeneratedImageSummary>, String> {
    video
        .validated_reference_images(user_id, project_id, chapter_id, Some(&json!(ids)))
        .await
        .map_err(|error| error.to_string())
}
