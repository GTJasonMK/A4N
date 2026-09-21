mod characters;
mod counts;
mod materials;
mod objects;
mod scenes;

use self::{
    characters::add_character_profile_nodes,
    counts::{material_image_counts_by_profile, portrait_counts_by_profile},
    objects::add_object_profile_nodes,
    scenes::add_scene_profile_nodes,
};
use crate::project_dependency_graph::GraphBuilder;
use crate::project_dependency_graph::graph::{
    CharacterProfileGraphRow, GeneratedImageGraphRow, ObjectProfileGraphRow, PortraitGraphRow,
    SceneProfileGraphRow,
};

pub(in crate::project_dependency_graph) fn add_profile_nodes(
    graph: &mut GraphBuilder,
    character_profiles: &[CharacterProfileGraphRow],
    object_profiles: &[ObjectProfileGraphRow],
    scene_profiles: &[SceneProfileGraphRow],
    portraits: &[PortraitGraphRow],
    generated_images: &[GeneratedImageGraphRow],
) {
    let portrait_counts = portrait_counts_by_profile(portraits);
    let object_image_counts = material_image_counts_by_profile(
        generated_images,
        crate::profile_material::PROFILE_KIND_OBJECT,
    );
    let scene_image_counts = material_image_counts_by_profile(
        generated_images,
        crate::profile_material::PROFILE_KIND_SCENE,
    );

    add_character_profile_nodes(graph, character_profiles, &portrait_counts);
    add_object_profile_nodes(graph, object_profiles, &object_image_counts);
    add_scene_profile_nodes(graph, scene_profiles, &scene_image_counts);
}

#[cfg(test)]
mod tests;
