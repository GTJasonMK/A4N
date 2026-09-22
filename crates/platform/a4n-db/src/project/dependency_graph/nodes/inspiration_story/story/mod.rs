use crate::project::dependency_graph::GraphBuilder;
use crate::project::dependency_graph::graph::{
    BlueprintGraphRow, BlueprintVersionGraphRow, ChapterGraphRow, ChapterOutlineGraphRow,
    PartOutlineGraphRow,
};

mod blueprint;
use blueprint::add_blueprint_nodes;

mod chapters;
use chapters::add_chapter_nodes;

mod outlines;
use outlines::add_outline_nodes;

pub(in crate::project::dependency_graph) fn add_story_nodes(
    graph: &mut GraphBuilder,
    blueprints: &[BlueprintGraphRow],
    blueprint_versions: &[BlueprintVersionGraphRow],
    part_outlines: &[PartOutlineGraphRow],
    chapter_outlines: &[ChapterOutlineGraphRow],
    chapters: &[ChapterGraphRow],
) {
    add_blueprint_nodes(graph, blueprints, blueprint_versions);
    add_outline_nodes(graph, part_outlines, chapter_outlines);
    add_chapter_nodes(graph, chapters);
}
