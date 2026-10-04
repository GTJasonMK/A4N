use std::collections::{HashMap, HashSet};

use crate::project::{ProjectFamilyPackage, snapshot_profile_key};

pub(super) struct GenerationReferenceContext<'a> {
    pub(super) specs: HashMap<i64, &'a a4n_domain::project::ProjectFamilyGenerationSpecRecord>,
    pub(super) source_snapshots:
        HashMap<i64, &'a a4n_domain::project::ProjectFamilySourceSnapshotRecord>,
    pub(super) snapshot_chapters:
        HashMap<i64, &'a a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord>,
    pub(super) snapshot_profiles:
        HashMap<(i64, String), &'a a4n_domain::project::ProjectFamilySourceSnapshotProfileRecord>,
    pub(super) text_anchors: HashMap<i64, &'a a4n_domain::project::ProjectFamilyTextAnchorRecord>,
    pub(super) media_assets: HashMap<i64, &'a a4n_domain::project::ProjectFamilyMediaAssetRecord>,
    pub(super) chapter_video_prompt_ids: HashSet<i64>,
    pub(super) video_segment_ids: HashSet<i64>,
    pub(super) assembly_timeline_ids: HashSet<i64>,
    pub(super) galgame_scripts:
        HashMap<i64, &'a a4n_domain::project::ProjectFamilyGalgameScriptRecord>,
    pub(super) galgame_scenes:
        HashMap<i64, &'a a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
    pub(super) galgame_lines: HashMap<i64, &'a a4n_domain::project::ProjectFamilyGalgameLineRecord>,
}

impl<'a> GenerationReferenceContext<'a> {
    pub(super) fn new(package: &'a ProjectFamilyPackage) -> Self {
        Self {
            specs: package
                .generation_specs
                .iter()
                .map(|spec| (spec.id, spec))
                .collect(),
            source_snapshots: package
                .source_snapshots
                .iter()
                .map(|snapshot| (snapshot.id, snapshot))
                .collect(),
            snapshot_chapters: package
                .source_snapshot_chapters
                .iter()
                .map(|chapter| (chapter.id, chapter))
                .collect(),
            snapshot_profiles: package
                .source_snapshot_profiles
                .iter()
                .map(|profile| {
                    (
                        (
                            profile.snapshot_id,
                            snapshot_profile_key(&profile.profile_kind, profile.source_profile_id),
                        ),
                        profile,
                    )
                })
                .collect(),
            text_anchors: package
                .text_anchors
                .iter()
                .map(|anchor| (anchor.id, anchor))
                .collect(),
            media_assets: package
                .media_assets
                .iter()
                .map(|asset| (asset.id, asset))
                .collect(),
            chapter_video_prompt_ids: package
                .chapter_video_prompts
                .iter()
                .map(|prompt| prompt.id)
                .collect(),
            video_segment_ids: package
                .video_segments
                .iter()
                .map(|segment| segment.id)
                .collect(),
            assembly_timeline_ids: package
                .assembly_timelines
                .iter()
                .map(|timeline| timeline.id)
                .collect(),
            galgame_scripts: package
                .galgame_scripts
                .iter()
                .map(|script| (script.id, script))
                .collect(),
            galgame_scenes: package
                .galgame_scenes
                .iter()
                .map(|scene| (scene.id, scene))
                .collect(),
            galgame_lines: package
                .galgame_lines
                .iter()
                .map(|line| (line.id, line))
                .collect(),
        }
    }
}
