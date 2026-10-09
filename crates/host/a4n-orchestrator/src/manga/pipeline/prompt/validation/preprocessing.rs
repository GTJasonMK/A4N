use a4n_domain::manga::MangaPreprocessingV3;
use a4n_error::AppError;
use serde_json::Value;

pub(in crate::manga) fn validate_manga_preprocessing(
    value: &Value,
) -> Result<MangaPreprocessingV3, AppError> {
    let preprocessing: MangaPreprocessingV3 = serde_json::from_value(value.clone())
        .map_err(|error| AppError::InvalidArgument(format!("漫画预处理合同无效: {error}")))?;
    preprocessing
        .validate()
        .map_err(AppError::InvalidArgument)?;
    Ok(preprocessing)
}
