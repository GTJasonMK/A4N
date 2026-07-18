use super::*;

#[cfg(test)]
mod analysis_order_tests {
    use super::*;

    #[test]
    fn subprogress_mapping_scales_into_parent_span() {
        assert_eq!(map_subprogress(0, 100, 58, 68), 58);
        assert_eq!(map_subprogress(50, 100, 58, 68), 63);
        assert_eq!(map_subprogress(100, 100, 58, 68), 68);
        assert_eq!(map_subprogress(6, 6, 78, 88), 88);
    }

    #[test]
    fn blocked_scene_profile_sync_preview_steps_cover_scene_sync_chain() {
        let steps =
            blocked_scene_profile_sync_chapter_analysis_steps(3, "结构化分析保存后才能构造");

        assert_eq!(steps.len(), 4);
        assert!(steps.iter().all(|step| step.state == "blocked"));
        assert_eq!(
            steps
                .iter()
                .filter_map(|step| step.prompt_name.as_deref())
                .collect::<Vec<_>>(),
            vec![
                "scene_profile_candidates",
                "scene_occurrence_extraction",
                "scene_transition_extraction",
                "scene_entity_state_extraction",
            ]
        );
        assert_eq!(steps[0].key, "scene_profile_sync_ch3_scenes");
    }
}
