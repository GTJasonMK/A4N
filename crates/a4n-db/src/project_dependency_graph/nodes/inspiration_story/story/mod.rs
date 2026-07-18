use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::{blueprint_version, novel_blueprint},
};

use crate::project_dependency_graph::GraphBuilder;

mod blueprint;
use blueprint::add_blueprint_nodes;

mod chapters;
use chapters::add_chapter_nodes;

mod outlines;
use outlines::add_outline_nodes;

pub(in crate::project_dependency_graph) fn add_story_nodes(
    graph: &mut GraphBuilder,
    blueprints: &[novel_blueprint::Model],
    blueprint_versions: &[blueprint_version::Model],
    part_outlines: &[part_outline::Model],
    chapter_outlines: &[chapter_outline::Model],
    chapters: &[chapter::Model],
) {
    add_blueprint_nodes(graph, blueprints, blueprint_versions);
    add_outline_nodes(graph, part_outlines, chapter_outlines);
    add_chapter_nodes(graph, chapters);
}
