use std::collections::HashSet;

pub(super) struct ManifestIds {
    pub(super) graph_ids: HashSet<i64>,
    pub(super) node_ids: HashSet<i64>,
    pub(super) text_asset_ids: HashSet<i64>,
    pub(super) generation_spec_ids: HashSet<i64>,
    pub(super) prompt_artifact_ids: HashSet<i64>,
    pub(super) image_ids: HashSet<i64>,
    pub(super) video_ids: HashSet<i64>,
    pub(super) media_asset_ids: HashSet<i64>,
    pub(super) video_segment_ids: HashSet<i64>,
    pub(super) assembly_timeline_ids: HashSet<i64>,
    pub(super) assembly_clip_ids: HashSet<i64>,
}
