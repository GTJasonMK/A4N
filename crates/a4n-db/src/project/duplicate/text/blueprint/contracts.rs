use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};

pub(in crate::project::duplicate::text) struct ProjectBlueprintGraph {
    pub(super) blueprint: novel_blueprint::Model,
    pub(super) versions: Vec<blueprint_version::Model>,
    pub(super) characters: Vec<blueprint_character::Model>,
    pub(super) relationships: Vec<blueprint_relationship::Model>,
}
