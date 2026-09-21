use super::context::current_snapshot;
use crate::runtime_config::RuntimeConfigSnapshot;

pub fn uses_system_default_llm_template() -> bool {
    current_snapshot_default_source_is_system(|snapshot| snapshot.llm_default_source)
}

pub fn uses_system_default_image_template() -> bool {
    current_snapshot_default_source_is_system(|snapshot| snapshot.image_default_source)
}

pub fn uses_system_default_video_template() -> bool {
    current_snapshot_default_source_is_system(|snapshot| snapshot.video_default_source)
}

fn current_snapshot_default_source_is_system(
    source: impl FnOnce(RuntimeConfigSnapshot) -> Option<String>,
) -> bool {
    current_snapshot()
        .and_then(source)
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("system"))
}
