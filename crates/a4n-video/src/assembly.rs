//! Pure validation and normalization for assembled video media facts.

pub fn assembly_media_properties<I>(clips: I) -> Result<(i32, i32, i32, f32), String>
where
    I: IntoIterator<Item = (i64, f32, i32, i32, i32)>,
{
    let mut clips = clips.into_iter();
    let (first_id, first_duration, first_width, first_height, first_fps) = clips
        .next()
        .ok_or_else(|| "成片至少需要一个视频片段".to_string())?;
    validate_assembly_media_properties(
        first_id,
        first_duration,
        first_width,
        first_height,
        first_fps,
    )?;
    let width = even_dimension(first_id, first_width, "宽度")?;
    let height = even_dimension(first_id, first_height, "高度")?;
    let mut duration_secs = first_duration;
    for (video_id, duration, clip_width, clip_height, clip_fps) in clips {
        validate_assembly_media_properties(video_id, duration, clip_width, clip_height, clip_fps)?;
        duration_secs += duration;
        if !duration_secs.is_finite() {
            return Err("成片总时长超过可记录范围".to_string());
        }
    }
    Ok((width, height, first_fps, duration_secs))
}

fn validate_assembly_media_properties(
    video_id: i64,
    duration_secs: f32,
    width: i32,
    height: i32,
    fps: i32,
) -> Result<(), String> {
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Err(format!(
            "成片片段视频 #{video_id} 时长无效：{duration_secs}"
        ));
    }
    if width <= 0 || height <= 0 {
        return Err(format!(
            "成片片段视频 #{video_id} 尺寸无效：{width}x{height}"
        ));
    }
    if fps <= 0 {
        return Err(format!("成片片段视频 #{video_id} 帧率无效：{fps}"));
    }
    Ok(())
}

fn even_dimension(video_id: i64, value: i32, label: &str) -> Result<i32, String> {
    if value % 2 == 0 {
        Ok(value)
    } else {
        value
            .checked_add(1)
            .ok_or_else(|| format!("成片片段视频 #{video_id} {label}超过可编码范围：{value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::assembly_media_properties;

    #[test]
    fn media_properties_are_strict_and_normalized_once() {
        assert_eq!(
            assembly_media_properties([(1, 1.5, 1281, 721, 25), (2, 2.0, 640, 360, 30),]).unwrap(),
            (1282, 722, 25, 3.5)
        );
        assert!(
            assembly_media_properties(std::iter::empty::<(i64, f32, i32, i32, i32)>()).is_err()
        );
        assert!(assembly_media_properties([(1, 0.0, 1280, 720, 24)]).is_err());
    }
}
