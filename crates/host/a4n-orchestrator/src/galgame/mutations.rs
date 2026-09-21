mod choices;
mod lines;
mod scenes;

use std::collections::BTreeSet;

use a4n_db::galgame::{ReplaceGalgameChoiceRecordInput, UpdateGalgameSceneRecordInput};
use a4n_db_models::models::galgame::{galgame_choice, galgame_scene};
use a4n_error::AppError;

use super::{
    LINE_TYPE_CHOICE, OrchestratorContext, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN,
    UpdateGalgameChoicesInput, UpdateGalgameSceneInput, galgame_scene_record_error,
    normalize_optional_text, required_text, to_i32, validate_positive_id,
};
