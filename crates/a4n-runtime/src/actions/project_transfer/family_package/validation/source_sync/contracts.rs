use super::*;

pub(super) struct SourceSyncReferenceSets {
    pub(super) generation_spec_ids: HashSet<i64>,
    pub(super) source_snapshot_chapter_ids: HashSet<i64>,
    pub(super) source_snapshot_chapter_numbers: HashSet<i32>,
    pub(super) source_snapshot_chapter_contexts: HashMap<i64, (i64, i32)>,
    pub(super) text_anchor_source_snapshot_chapter_ids: HashMap<i64, i64>,
    pub(super) source_span_anchor_chapter_numbers: HashMap<i64, i32>,
    pub(super) chapter_ids: HashSet<i64>,
    pub(super) text_anchor_ids: HashSet<i64>,
    pub(super) character_profile_ids: HashSet<i64>,
    pub(super) object_profile_ids: HashSet<i64>,
    pub(super) scene_profile_ids: HashSet<i64>,
    pub(super) galgame_scene_ids: HashSet<i64>,
    pub(super) galgame_scene_source_snapshot_chapter_ids: HashMap<i64, i64>,
    pub(super) galgame_line_ids: HashSet<i64>,
    pub(super) galgame_line_scene_ids: HashMap<i64, i64>,
}

impl SourceSyncReferenceSets {
    pub(super) fn from_package(package: &ProjectFamilyPackage) -> Self {
        let snapshot_chapter_numbers = package
            .source_snapshot_chapters
            .iter()
            .map(|chapter| (chapter.id, chapter.chapter_number))
            .collect::<HashMap<_, _>>();
        Self {
            generation_spec_ids: package
                .generation_specs
                .iter()
                .map(|spec| spec.id)
                .collect(),
            source_snapshot_chapter_ids: package
                .source_snapshot_chapters
                .iter()
                .map(|chapter| chapter.id)
                .collect(),
            source_snapshot_chapter_numbers: package
                .source_snapshot_chapters
                .iter()
                .map(|chapter| chapter.chapter_number)
                .collect(),
            source_snapshot_chapter_contexts: package
                .source_snapshot_chapters
                .iter()
                .map(|chapter| {
                    (
                        chapter.id,
                        (chapter.source_chapter_id, chapter.chapter_number),
                    )
                })
                .collect(),
            text_anchor_source_snapshot_chapter_ids: package
                .text_anchors
                .iter()
                .filter_map(|anchor| {
                    anchor
                        .source_snapshot_chapter_id
                        .map(|chapter_id| (anchor.id, chapter_id))
                })
                .collect(),
            source_span_anchor_chapter_numbers: package
                .text_anchors
                .iter()
                .filter_map(|anchor| {
                    let snapshot_chapter_id = anchor.source_snapshot_chapter_id?;
                    let chapter_number = snapshot_chapter_numbers.get(&snapshot_chapter_id)?;
                    Some((anchor.id, *chapter_number))
                })
                .collect(),
            chapter_ids: package
                .text_core
                .chapters
                .iter()
                .map(|chapter| chapter.id)
                .collect(),
            text_anchor_ids: package
                .text_anchors
                .iter()
                .map(|anchor| anchor.id)
                .collect(),
            character_profile_ids: package
                .text_core
                .character_profiles
                .iter()
                .map(|profile| profile.id)
                .collect(),
            object_profile_ids: package
                .text_core
                .object_profiles
                .iter()
                .map(|profile| profile.id)
                .collect(),
            scene_profile_ids: package
                .text_core
                .scene_profiles
                .iter()
                .map(|profile| profile.id)
                .collect(),
            galgame_scene_ids: package
                .galgame_scenes
                .iter()
                .map(|scene| scene.id)
                .collect(),
            galgame_scene_source_snapshot_chapter_ids: package
                .galgame_scenes
                .iter()
                .map(|scene| (scene.id, scene.source_snapshot_chapter_id))
                .collect(),
            galgame_line_ids: package.galgame_lines.iter().map(|line| line.id).collect(),
            galgame_line_scene_ids: package
                .galgame_lines
                .iter()
                .map(|line| (line.id, line.scene_id))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RegenerateSceneStageOutputContext {
    pub(super) expected_galgame_scene_id: i64,
    pub(super) expected_source_snapshot_chapter_id: i64,
    pub(super) expected_source_chapter_id: i64,
    pub(super) expected_chapter_number: i32,
}
