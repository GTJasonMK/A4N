use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
    project::{project_source_snapshot, source_snapshot_chapter},
};
use std::collections::{HashMap, HashSet};

use super::super::contracts::MediaRequestRefContext;
use crate::project::family_package::ProjectFamilyPackage;

pub(super) struct MediaManifestContext<'a> {
    pub(super) media_assets_by_id: HashMap<i64, &'a media_asset::Model>,
    pub(super) generation_specs: HashMap<i64, &'a generation_spec::Model>,
    pub(super) prompt_artifacts: HashMap<i64, &'a prompt_artifact::Model>,
    pub(super) chapter_ids: HashSet<i64>,
    pub(super) character_profile_ids: HashSet<i64>,
    pub(super) object_profile_ids: HashSet<i64>,
    pub(super) scene_profile_ids: HashSet<i64>,
    pub(super) chapter_video_prompt_ids: HashSet<i64>,
    pub(super) generated_video_ids: HashSet<i64>,
    pub(super) video_segment_ids: HashSet<i64>,
    pub(super) assembly_timeline_ids: HashSet<i64>,
    pub(super) galgame_scene_ids: HashSet<i64>,
    pub(super) galgame_line_ids: HashSet<i64>,
    pub(super) galgame_scripts: HashMap<i64, &'a galgame_script::Model>,
    pub(super) galgame_scenes: HashMap<i64, &'a galgame_scene::Model>,
    pub(super) galgame_lines: HashMap<i64, &'a galgame_line::Model>,
    pub(super) generation_spec_ids: HashSet<i64>,
    pub(super) prompt_artifact_ids: HashSet<i64>,
    pub(super) source_snapshots: HashMap<i64, &'a project_source_snapshot::Model>,
    pub(super) source_snapshot_chapters: HashMap<i64, &'a source_snapshot_chapter::Model>,
}

impl<'a> MediaManifestContext<'a> {
    pub(super) fn new(package: &'a ProjectFamilyPackage) -> Self {
        let media_assets_by_id = package
            .media_assets
            .iter()
            .map(|asset| (asset.id, asset))
            .collect::<HashMap<_, _>>();
        let generation_specs = package
            .generation_specs
            .iter()
            .map(|spec| (spec.id, spec))
            .collect::<HashMap<_, _>>();
        let prompt_artifacts = package
            .prompt_artifacts
            .iter()
            .map(|artifact| (artifact.id, artifact))
            .collect::<HashMap<_, _>>();
        let chapter_ids = package
            .text_core
            .chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<HashSet<_>>();
        let character_profile_ids = package
            .text_core
            .character_profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<HashSet<_>>();
        let object_profile_ids = package
            .text_core
            .object_profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<HashSet<_>>();
        let scene_profile_ids = package
            .text_core
            .scene_profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<HashSet<_>>();
        let chapter_video_prompt_ids = package
            .chapter_video_prompts
            .iter()
            .map(|prompt| prompt.id)
            .collect::<HashSet<_>>();
        let generated_video_ids = package
            .generated_videos
            .iter()
            .map(|video| video.id)
            .collect::<HashSet<_>>();
        let video_segment_ids = package
            .video_segments
            .iter()
            .map(|segment| segment.id)
            .collect::<HashSet<_>>();
        let assembly_timeline_ids = package
            .assembly_timelines
            .iter()
            .map(|timeline| timeline.id)
            .collect::<HashSet<_>>();
        let galgame_scene_ids = package
            .galgame_scenes
            .iter()
            .map(|scene| scene.id)
            .collect::<HashSet<_>>();
        let galgame_line_ids = package
            .galgame_lines
            .iter()
            .map(|line| line.id)
            .collect::<HashSet<_>>();
        let galgame_scripts = package
            .galgame_scripts
            .iter()
            .map(|script| (script.id, script))
            .collect::<HashMap<_, _>>();
        let galgame_scenes = package
            .galgame_scenes
            .iter()
            .map(|scene| (scene.id, scene))
            .collect::<HashMap<_, _>>();
        let galgame_lines = package
            .galgame_lines
            .iter()
            .map(|line| (line.id, line))
            .collect::<HashMap<_, _>>();
        let generation_spec_ids = generation_specs.keys().copied().collect::<HashSet<_>>();
        let prompt_artifact_ids = prompt_artifacts.keys().copied().collect::<HashSet<_>>();
        let source_snapshots = package
            .source_snapshots
            .iter()
            .map(|snapshot| (snapshot.id, snapshot))
            .collect::<HashMap<_, _>>();
        let source_snapshot_chapters = package
            .source_snapshot_chapters
            .iter()
            .map(|chapter| (chapter.id, chapter))
            .collect::<HashMap<_, _>>();
        Self {
            media_assets_by_id,
            generation_specs,
            prompt_artifacts,
            chapter_ids,
            character_profile_ids,
            object_profile_ids,
            scene_profile_ids,
            chapter_video_prompt_ids,
            generated_video_ids,
            video_segment_ids,
            assembly_timeline_ids,
            galgame_scene_ids,
            galgame_line_ids,
            galgame_scripts,
            galgame_scenes,
            galgame_lines,
            generation_spec_ids,
            prompt_artifact_ids,
            source_snapshots,
            source_snapshot_chapters,
        }
    }

    pub(super) fn request_refs(&self) -> MediaRequestRefContext<'_> {
        MediaRequestRefContext {
            generation_specs: &self.generation_specs,
            generation_spec_ids: &self.generation_spec_ids,
            prompt_artifacts: &self.prompt_artifacts,
            prompt_artifact_ids: &self.prompt_artifact_ids,
            character_profile_ids: &self.character_profile_ids,
            video_segment_ids: &self.video_segment_ids,
            assembly_timeline_ids: &self.assembly_timeline_ids,
            galgame_scripts: &self.galgame_scripts,
            galgame_scenes: &self.galgame_scenes,
            galgame_lines: &self.galgame_lines,
        }
    }
}
