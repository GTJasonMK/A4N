use super::super::*;

pub(super) struct MediaRequestRefContext<'a> {
    pub(super) generation_specs: &'a HashMap<i64, &'a generation_spec::Model>,
    pub(super) generation_spec_ids: &'a HashSet<i64>,
    pub(super) prompt_artifacts: &'a HashMap<i64, &'a prompt_artifact::Model>,
    pub(super) prompt_artifact_ids: &'a HashSet<i64>,
    pub(super) character_profile_ids: &'a HashSet<i64>,
    pub(super) video_segment_ids: &'a HashSet<i64>,
    pub(super) assembly_timeline_ids: &'a HashSet<i64>,
    pub(super) galgame_scripts: &'a HashMap<i64, &'a galgame_script::Model>,
    pub(super) galgame_scenes: &'a HashMap<i64, &'a galgame_scene::Model>,
    pub(super) galgame_lines: &'a HashMap<i64, &'a galgame_line::Model>,
}

pub(super) struct MediaProvenanceRefContext<'a> {
    pub(super) media_assets: &'a HashMap<i64, &'a media_asset::Model>,
    pub(super) request: MediaRequestRefContext<'a>,
}
