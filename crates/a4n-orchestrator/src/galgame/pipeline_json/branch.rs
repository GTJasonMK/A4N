use a4n_core::error::AppError;
use serde_json::Value;

use super::contracts::*;
use super::rules::*;

pub fn branch_design_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameBranchDesign, AppError> {
    let raw: BranchDesignRaw = from_value(value, context)?;
    let choices = raw
        .choices
        .into_iter()
        .map(|choice| {
            require_positive(
                choice.after_beat_ref.scene_index,
                &format!("{context}.choices[].after_beat_ref.scene_index"),
            )?;
            if choice.after_beat_ref.shot_index == 0 {
                return Err(invalid(format!(
                    "{context}.choices[].after_beat_ref.shot_index 必须大于 0"
                )));
            }
            if choice.after_beat_ref.beat_index == 0 {
                return Err(invalid(format!(
                    "{context}.choices[].after_beat_ref.beat_index 必须大于 0"
                )));
            }
            if !(2..=3).contains(&choice.options.len()) {
                return Err(invalid(format!(
                    "{context}.choices[].options 必须包含 2 到 3 个选项"
                )));
            }
            let options = choice
                .options
                .into_iter()
                .map(|option| {
                    require_positive(
                        option.returns_to_scene_index,
                        &format!("{context}.choices[].options[].returns_to_scene_index"),
                    )?;
                    Ok(GalgamePipelineChoiceOptionPlan {
                        branch_id: required_text(
                            option.branch_id,
                            &format!("{context}.choices[].options[].branch_id"),
                        )?,
                        option_text: required_text(
                            option.option_text,
                            &format!("{context}.choices[].options[].option_text"),
                        )?,
                        branch_title: required_text(
                            option.branch_title,
                            &format!("{context}.choices[].options[].branch_title"),
                        )?,
                        branch_objective: required_text(
                            option.branch_objective,
                            &format!("{context}.choices[].options[].branch_objective"),
                        )?,
                        dramatic_action: required_text(
                            option.dramatic_action,
                            &format!("{context}.choices[].options[].dramatic_action"),
                        )?,
                        reaction: required_text(
                            option.reaction,
                            &format!("{context}.choices[].options[].reaction"),
                        )?,
                        player_knowledge: normalize_text_vec(
                            option.player_knowledge,
                            &format!("{context}.choices[].options[].player_knowledge"),
                        )?,
                        returns_to_scene_index: option.returns_to_scene_index,
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            Ok(GalgamePipelineChoicePlan {
                after_beat_ref: choice.after_beat_ref,
                question: required_text(choice.question, &format!("{context}.choices[].question"))?,
                options,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(GalgameBranchDesign { choices })
}
