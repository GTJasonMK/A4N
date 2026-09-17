use super::super::contracts::ImportedVideoSource;
use crate::AppState;
use crate::actions::video_records::probe_video_properties;
use a4n_media_contract::MediaRoot;
use a4n_video_host_contract::ImportedVideoProperties;

pub(in crate::actions::video_records) async fn probe_imported_video_properties(
    state: &AppState,
    source: &ImportedVideoSource,
    label: &str,
) -> Result<ImportedVideoProperties, String> {
    let properties = probe_video_properties(state, &source.path, MediaRoot::Temp, label).await?;
    Ok(ImportedVideoProperties {
        width: properties.width,
        height: properties.height,
        duration_secs: properties.duration_secs,
        fps: properties.fps,
    })
}
