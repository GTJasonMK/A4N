use a4n_core::models::ai::image_gen_config;
use serde_json::Value;

use super::validation::optional_string_field;

pub(super) fn batch_image_dimensions(
    config: &image_gen_config::Model,
    item: &Value,
    metadata_json: Option<&Value>,
) -> Result<(i32, i32), String> {
    let Some(aspect_ratio) = batch_image_aspect_ratio(item, metadata_json)? else {
        return Ok((config.default_width, config.default_height));
    };
    image_dimensions_for_aspect_ratio(config.default_width, config.default_height, &aspect_ratio)
}

pub(super) fn batch_image_aspect_ratio(
    item: &Value,
    metadata_json: Option<&Value>,
) -> Result<Option<String>, String> {
    let top_level = optional_string_field(item, "aspect_ratio")?;
    let metadata = metadata_json
        .map(|metadata| {
            optional_aspect_ratio_field(metadata, "aspect_ratio", "metadata_json.aspect_ratio")
        })
        .transpose()?
        .flatten();
    let prompt_snapshot = metadata_json
        .and_then(|metadata| metadata.get("prompt_snapshot"))
        .map(|snapshot| {
            optional_aspect_ratio_field(
                snapshot,
                "aspect_ratio",
                "metadata_json.prompt_snapshot.aspect_ratio",
            )
        })
        .transpose()?
        .flatten();
    let generation_context = metadata_json
        .and_then(|metadata| metadata.get("generation_context"))
        .map(|context| {
            optional_aspect_ratio_field(
                context,
                "aspect_ratio",
                "metadata_json.generation_context.aspect_ratio",
            )
        })
        .transpose()?
        .flatten();
    Ok([top_level, metadata, prompt_snapshot, generation_context]
        .into_iter()
        .flatten()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .map(ToString::to_string))
}

fn optional_aspect_ratio_field<'a>(
    container: &'a Value,
    field: &str,
    label: &str,
) -> Result<Option<&'a str>, String> {
    let Some(value) = container.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| format!("{label} 必须是字符串"))
}

pub(super) fn image_dimensions_for_aspect_ratio(
    default_width: i32,
    default_height: i32,
    aspect_ratio: &str,
) -> Result<(i32, i32), String> {
    let (ratio_width, ratio_height) = parse_image_aspect_ratio(aspect_ratio)?;
    let long_edge = i64::from(default_width.max(default_height).max(1));
    let (width, height) = if ratio_width >= ratio_height {
        (
            long_edge,
            rounded_ratio_dimension(long_edge, ratio_height, ratio_width),
        )
    } else {
        (
            rounded_ratio_dimension(long_edge, ratio_width, ratio_height),
            long_edge,
        )
    };
    let width = i32::try_from(width.max(1))
        .map_err(|_| format!("图片宽度超过 i32 范围：{}", width.max(1)))?;
    let height = i32::try_from(height.max(1))
        .map_err(|_| format!("图片高度超过 i32 范围：{}", height.max(1)))?;
    Ok((width, height))
}

fn parse_image_aspect_ratio(aspect_ratio: &str) -> Result<(i64, i64), String> {
    let normalized = aspect_ratio.trim().replace('：', ":").replace('／', "/");
    let Some((left, right)) = normalized
        .split_once(':')
        .or_else(|| normalized.split_once('/'))
    else {
        return Err(format!(
            "图片 aspect_ratio 无效：{aspect_ratio}，格式应为 16:9、9:16 或 1:1"
        ));
    };
    let ratio_width = left
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("图片 aspect_ratio 无效：{aspect_ratio}，宽高比必须是正整数"))?;
    let ratio_height = right
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("图片 aspect_ratio 无效：{aspect_ratio}，宽高比必须是正整数"))?;
    if ratio_width <= 0 || ratio_height <= 0 {
        return Err(format!(
            "图片 aspect_ratio 无效：{aspect_ratio}，宽高比必须大于 0"
        ));
    }
    Ok((ratio_width, ratio_height))
}

fn rounded_ratio_dimension(long_edge: i64, numerator: i64, denominator: i64) -> i64 {
    ((long_edge * numerator) + (denominator / 2)) / denominator
}
