use super::super::super::apply_attribute_to_state;
use serde_json::Value;

pub(in crate::actions::character_ai) fn fold_attribute_event_tuples(
    baseline_state: Value,
    events: &mut [(i32, i64, &str, &str, &str, Value)],
    until_chapter: i32,
) -> Value {
    events.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let mut state = baseline_state;
    for (chapter_number, _, category, key, operation, new_value) in events.iter() {
        if *chapter_number > until_chapter {
            break;
        }
        apply_attribute_to_state(&mut state, category, key, operation, new_value.clone())
            .expect("test state must be canonical");
    }
    state
}
