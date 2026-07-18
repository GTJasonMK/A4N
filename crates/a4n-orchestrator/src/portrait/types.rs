use a4n_core::error::AppError;

enum PortraitType {
    FullBody,
    HalfBody,
    Face,
    Chibi,
}

impl PortraitType {
    fn description(&self) -> &'static str {
        match self {
            Self::FullBody => "人物三视图与拆解设定图（正面、侧面、背面，全身，含服饰配饰拆解）",
            Self::HalfBody => {
                "人物三视图与拆解设定图（正面、侧面、背面，重点拆解服饰层级与配饰细节）"
            }
            Self::Face => "人物三视图与拆解设定图（含面部与发型拆解参考）",
            Self::Chibi => "Q版人物三视图与拆解设定图",
        }
    }
}

fn parse_portrait_type(portrait_type: &str) -> Result<PortraitType, AppError> {
    match portrait_type.trim() {
        "" => Err(AppError::InvalidArgument("portrait_type 不能为空".into())),
        "full_body" => Ok(PortraitType::FullBody),
        "half_body" => Ok(PortraitType::HalfBody),
        "face" => Ok(PortraitType::Face),
        "chibi" => Ok(PortraitType::Chibi),
        other => Err(AppError::InvalidArgument(format!(
            "不支持的立绘类型: {other}"
        ))),
    }
}

pub(super) fn portrait_type_description(portrait_type: &str) -> Result<&'static str, AppError> {
    Ok(parse_portrait_type(portrait_type)?.description())
}
