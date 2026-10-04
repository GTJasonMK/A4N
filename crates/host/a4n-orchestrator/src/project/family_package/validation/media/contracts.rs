use super::super::*;

pub(super) struct MediaRequestRefContext<'a> {
    pub(super) generation_specs:
        &'a HashMap<i64, &'a a4n_domain::project::ProjectFamilyGenerationSpecRecord>,
    pub(super) generation_spec_ids: &'a HashSet<i64>,
    pub(super) prompt_artifacts:
        &'a HashMap<i64, &'a a4n_domain::project::ProjectFamilyPromptArtifactRecord>,
    pub(super) prompt_artifact_ids: &'a HashSet<i64>,
    pub(super) character_profile_ids: &'a HashSet<i64>,
    pub(super) video_segment_ids: &'a HashSet<i64>,
    pub(super) assembly_timeline_ids: &'a HashSet<i64>,
    pub(super) galgame_scripts:
        &'a HashMap<i64, &'a a4n_domain::project::ProjectFamilyGalgameScriptRecord>,
    pub(super) galgame_scenes:
        &'a HashMap<i64, &'a a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
    pub(super) galgame_lines:
        &'a HashMap<i64, &'a a4n_domain::project::ProjectFamilyGalgameLineRecord>,
}

pub(super) struct MediaProvenanceRefContext<'a> {
    pub(super) media_assets:
        &'a HashMap<i64, &'a a4n_domain::project::ProjectFamilyMediaAssetRecord>,
    pub(super) request: MediaRequestRefContext<'a>,
}
