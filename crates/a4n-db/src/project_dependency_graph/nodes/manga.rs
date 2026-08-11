use super::*;
use serde_json::json;

pub(in crate::project_dependency_graph) fn add_manga_nodes(
    graph: &mut GraphBuilder,
    chapters: &[chapter::Model],
    manga_prompts: &[chapter_manga_prompt::Model],
    generated_images: &[generated_image::Model],
) {
    let prompt_stats = manga_prompt_stats(manga_prompts);
    graph.add_node(node(
        "manga_prompts",
        "prompt",
        "chapter_manga_prompt",
        "漫画中间提示词",
        prompt_stats.visible_count,
        8,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some("漫画整页绘图提示词。".to_string()),
            details: json!({
                "chapter_count": prompt_stats.chapter_count(),
            }),
        },
    ));
    if !chapters.is_empty() {
        graph.add_edge(
            "chapters",
            "manga_prompts",
            "derives",
            prompt_stats.visible_count,
        );
    }
    if graph.node_count("character_profiles") > 0 {
        graph.add_edge(
            "character_profiles",
            "manga_prompts",
            "references",
            prompt_stats.visible_count,
        );
    }

    let image_type_manga_count = generated_images
        .iter()
        .filter(|row| row.image_type == "manga_page")
        .count() as u64;
    let manga_image_count = prompt_stats.linked_image_count.max(image_type_manga_count);
    graph.add_node(node(
        "manga_images",
        "image",
        "manga_page_image",
        "漫画图片",
        manga_image_count,
        9,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("由整页漫画提示词生成或绑定的页面图片。".to_string()),
            details: json!({
                "linked_prompt_count": prompt_stats.linked_image_count,
                "image_type_count": image_type_manga_count,
            }),
        },
    ));
    graph.add_edge(
        "manga_prompts",
        "manga_images",
        "generates",
        manga_image_count,
    );
}

struct MangaPromptStats {
    visible_count: u64,
    linked_image_count: u64,
    chapter_ids: BTreeSet<i64>,
}

impl MangaPromptStats {
    fn chapter_count(&self) -> usize {
        self.chapter_ids.len()
    }
}

fn manga_prompt_stats(manga_prompts: &[chapter_manga_prompt::Model]) -> MangaPromptStats {
    let mut stats = MangaPromptStats {
        visible_count: 0,
        linked_image_count: 0,
        chapter_ids: BTreeSet::new(),
    };

    for row in manga_prompts {
        stats.visible_count += 1;
        if row.generated_image_id.is_some() {
            stats.linked_image_count += 1;
        }
        stats.chapter_ids.insert(row.chapter_id);
    }

    stats
}
