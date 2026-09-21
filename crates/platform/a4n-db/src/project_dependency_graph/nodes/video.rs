use crate::project_dependency_graph::builder::GraphBuilder;
use crate::project_dependency_graph::graph::{
    ChapterGraphRow, GeneratedImageGraphRow, GeneratedVideoGraphRow, ShootingScriptGraphRow,
    VideoPromptGraphRow, VisualPlanGraphRow,
};

mod outputs;
mod planning;
mod stats;

use outputs::add_video_output_nodes;
use planning::add_video_planning_nodes;
use stats::video_prompt_stats;

pub(in crate::project_dependency_graph) fn add_video_nodes(
    graph: &mut GraphBuilder,
    chapters: &[ChapterGraphRow],
    visual_plans: &[VisualPlanGraphRow],
    shooting_scripts: &[ShootingScriptGraphRow],
    video_prompts: &[VideoPromptGraphRow],
    generated_images: &[GeneratedImageGraphRow],
    generated_videos: &[GeneratedVideoGraphRow],
) {
    let prompt_stats = video_prompt_stats(video_prompts);
    add_video_planning_nodes(
        graph,
        chapters,
        visual_plans,
        shooting_scripts,
        &prompt_stats,
    );
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
