use a4n_core::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter, chapter_shooting_script, chapter_video_prompt},
};

use crate::project_dependency_graph::builder::GraphBuilder;

mod outputs;
mod planning;
mod stats;

use outputs::add_video_output_nodes;
use planning::add_video_planning_nodes;
use stats::video_prompt_stats;

pub(in crate::project_dependency_graph) fn add_video_nodes(
    graph: &mut GraphBuilder,
    chapters: &[chapter::Model],
    shooting_scripts: &[chapter_shooting_script::Model],
    video_prompts: &[chapter_video_prompt::Model],
    generated_images: &[generated_image::Model],
    generated_videos: &[generated_video::Model],
) {
    let prompt_stats = video_prompt_stats(video_prompts);
    add_video_planning_nodes(graph, chapters, shooting_scripts, &prompt_stats);
    add_video_output_nodes(
        graph,
        chapters.len() as u64,
        generated_images,
        generated_videos,
        &prompt_stats,
    );
}

#[cfg(test)]
mod tests;
