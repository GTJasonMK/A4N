#[derive(Debug, Clone, Default)]
pub(in crate::project::rollback_impact) struct ProjectRollbackImpactFacts {
    pub(in crate::project::rollback_impact) chapter_numbers: Vec<i32>,
    pub(in crate::project::rollback_impact) outline_numbers: Vec<i32>,
    pub(in crate::project::rollback_impact) part_numbers: Vec<i32>,
    pub(in crate::project::rollback_impact) media_image_count: i32,
    pub(in crate::project::rollback_impact) media_video_count: i32,
    pub(in crate::project::rollback_impact) task_count: i32,
    pub(in crate::project::rollback_impact) object_profile_count: i32,
    pub(in crate::project::rollback_impact) scene_profile_count: i32,
    pub(in crate::project::rollback_impact) scene_occurrence_count: i32,
    pub(in crate::project::rollback_impact) scene_transition_count: i32,
    pub(in crate::project::rollback_impact) scene_entity_state_count: i32,
    pub(in crate::project::rollback_impact) blueprint_character_count: i32,
    pub(in crate::project::rollback_impact) blueprint_relationship_count: i32,
    pub(in crate::project::rollback_impact) blueprint_version_count: i32,
    pub(in crate::project::rollback_impact) novel_blueprint_count: i32,
}
