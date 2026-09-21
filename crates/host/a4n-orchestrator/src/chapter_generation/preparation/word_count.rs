use a4n_db_models::models::chapter::chapter_outline;
use serde::Serialize;

const DEFAULT_CHAPTER_WORD_COUNT_MIN: i32 = 4_500;
const DEFAULT_CHAPTER_WORD_COUNT_MAX: i32 = 6_000;
const WORD_COUNT_TARGET_MIN_RATIO_NUMERATOR: i32 = 9;
const WORD_COUNT_TARGET_MIN_RATIO_DENOMINATOR: i32 = 10;
const WORD_COUNT_TARGET_MAX_RATIO_NUMERATOR: i32 = 11;
const WORD_COUNT_TARGET_MAX_RATIO_DENOMINATOR: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ChapterGenerationWordCountRange {
    pub min: i32,
    pub max: i32,
}

impl ChapterGenerationWordCountRange {
    pub(super) fn label(self) -> String {
        format!("{}-{} 字", self.min, self.max)
    }
}

pub fn resolve_generation_word_count_range(
    outline: Option<&chapter_outline::Model>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
) -> Result<ChapterGenerationWordCountRange, String> {
    match (min_word_count, max_word_count) {
        (Some(min), Some(max)) => validate_generation_word_count_range(min, max),
        (Some(_), None) | (None, Some(_)) => {
            Err("正文字数范围必须同时提供最小字数和最大字数".to_string())
        }
        (None, None) => default_generation_word_count_range(outline),
    }
}

fn default_generation_word_count_range(
    outline: Option<&chapter_outline::Model>,
) -> Result<ChapterGenerationWordCountRange, String> {
    let Some(target) = outline.and_then(|outline| outline.word_count_target) else {
        return Ok(ChapterGenerationWordCountRange {
            min: DEFAULT_CHAPTER_WORD_COUNT_MIN,
            max: DEFAULT_CHAPTER_WORD_COUNT_MAX,
        });
    };
    if target <= 0 {
        return Ok(ChapterGenerationWordCountRange {
            min: DEFAULT_CHAPTER_WORD_COUNT_MIN,
            max: DEFAULT_CHAPTER_WORD_COUNT_MAX,
        });
    }
    let min = scaled_i32_floor(
        target,
        WORD_COUNT_TARGET_MIN_RATIO_NUMERATOR,
        WORD_COUNT_TARGET_MIN_RATIO_DENOMINATOR,
    )?
    .max(1);
    let max = scaled_i32_ceil(
        target,
        WORD_COUNT_TARGET_MAX_RATIO_NUMERATOR,
        WORD_COUNT_TARGET_MAX_RATIO_DENOMINATOR,
    )?
    .max(min);
    Ok(ChapterGenerationWordCountRange { min, max })
}

fn scaled_i32_floor(value: i32, numerator: i32, denominator: i32) -> Result<i32, String> {
    let scaled = i64::from(value) * i64::from(numerator) / i64::from(denominator);
    i32::try_from(scaled).map_err(|_| {
        format!("章节目标字数比例计算超过 i32 上限：{value} * {numerator} / {denominator}")
    })
}

fn scaled_i32_ceil(value: i32, numerator: i32, denominator: i32) -> Result<i32, String> {
    let scaled = (i64::from(value) * i64::from(numerator) + i64::from(denominator) - 1)
        / i64::from(denominator);
    i32::try_from(scaled).map_err(|_| {
        format!("章节目标字数比例计算超过 i32 上限：{value} * {numerator} / {denominator}")
    })
}

fn validate_generation_word_count_range(
    min: i32,
    max: i32,
) -> Result<ChapterGenerationWordCountRange, String> {
    if min <= 0 {
        return Err("正文字数最小值必须大于 0".to_string());
    }
    if max <= 0 {
        return Err("正文字数最大值必须大于 0".to_string());
    }
    if max < min {
        return Err("正文字数最大值不能小于最小值".to_string());
    }
    Ok(ChapterGenerationWordCountRange { min, max })
}

pub(super) fn format_generation_word_count_requirement(
    range: ChapterGenerationWordCountRange,
) -> String {
    format!(
        "## 正文字数要求\n最小字数: {}\n最大字数: {}\n要求: 章节正文纯文本必须控制在 {} 之间；按正文字符数估算，不要输出 word_count 字段、JSON 字段名、标题字段或摘要字段，不要用重复、注水或解释性段落凑字。",
        range.min,
        range.max,
        range.label()
    )
}
