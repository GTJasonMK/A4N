mod characters;
mod counts;
mod materials;
mod objects;
mod scenes;

use a4n_core::models::{
    ai::generated_image,
    asset::object_profile,
    character::{character_portrait, protagonist_profile},
    scene::scene_profile,
};

use self::{
    characters::add_character_profile_nodes,
    counts::{material_image_counts_by_profile, portrait_counts_by_profile},
    objects::add_object_profile_nodes,
    scenes::add_scene_profile_nodes,
};
use crate::project_dependency_graph::GraphBuilder;

pub(in crate::project_dependency_graph) fn add_profile_nodes(
    graph: &mut GraphBuilder,
    character_profiles: &[protagonist_profile::Model],
    object_profiles: &[object_profile::Model],
    scene_profiles: &[scene_profile::Model],
    portraits: &[character_portrait::Model],
    generated_images: &[generated_image::Model],
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
