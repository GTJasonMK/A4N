use super::super::contracts::ImportedVideoSource;
use crate::actions::video_records::probe_video_properties;
use a4n_video_host_contract::ImportedVideoProperties;

pub(in crate::actions::video_records) fn probe_imported_video_properties(
    source: &ImportedVideoSource,
    label: &str,
) -> Result<ImportedVideoProperties, String> {
    let properties = probe_video_properties(&source.path, label)?;
    Ok(ImportedVideoProperties {
        width: properties.width,
        height: properties.height,
        duration_secs: properties.duration_secs,
        fps: properties.fps,
    })
}
