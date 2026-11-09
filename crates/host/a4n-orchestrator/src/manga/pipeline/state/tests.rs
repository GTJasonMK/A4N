use super::manga_page_pipeline_call_order;
use super::{MangaPagePlanningStep, manga_page_planning_total_steps};

#[test]
fn planning_calls_are_page_scoped_and_disjoint_from_downstream_calls() {
    let mut keys = std::collections::HashSet::new();
    let mut orders = std::collections::HashSet::new();
    for step in std::iter::once(MangaPagePlanningStep::Outline)
        .chain((0..100).map(MangaPagePlanningStep::Content))
    {
        assert!(keys.insert(step.call_key(9)));
        let order = step.call_order().unwrap();
        assert!(order < manga_page_pipeline_call_order(0, 0).unwrap());
        assert!(orders.insert(order));
    }
    assert_eq!(manga_page_planning_total_steps(1).unwrap(), 3);
    assert_eq!(manga_page_planning_total_steps(100).unwrap(), 102);
    assert!(manga_page_planning_total_steps(0).is_err());
    assert!(manga_page_planning_total_steps(101).is_err());
    assert!(MangaPagePlanningStep::Content(100).call_order().is_err());
}

#[test]
fn each_page_stage_has_a_stable_unique_call_order() {
    assert_eq!(manga_page_pipeline_call_order(0, 0).unwrap(), 800);
    assert_eq!(manga_page_pipeline_call_order(0, 2).unwrap(), 802);
    assert_eq!(manga_page_pipeline_call_order(0, 3).unwrap(), 803);
    assert_eq!(manga_page_pipeline_call_order(1, 0).unwrap(), 804);
    assert!(manga_page_pipeline_call_order(i32::MAX as usize, 0).is_err());
}
