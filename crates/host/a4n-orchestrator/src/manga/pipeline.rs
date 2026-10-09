pub(super) mod json;
pub(super) mod persist;
pub(super) mod preview;
pub(super) mod prompt;
pub(super) mod run {
    mod completion;
    mod contracts;
    mod drawing_prompt;
    mod page;
    mod page_slots;
    mod preprocessing;
    mod pipeline {
        mod execution;
    }
    mod progress;
    mod single_stage;
}
pub(super) mod state;
