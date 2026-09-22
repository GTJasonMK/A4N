//! Pure provider lifecycle validation rules.

pub fn ensure_provider_generation_run_updated(
    video_id: i64,
    updated_run_count: usize,
    action: &str,
) -> Result<(), String> {
    if updated_run_count == 0 {
        return Err(format!(
            "无法{action}：视频 #{video_id} 缺少可更新的 generation_run"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ensure_provider_generation_run_updated;

    #[test]
    fn provider_lifecycle_requires_an_updated_generation_run() {
        assert!(ensure_provider_generation_run_updated(1, 1, "完成").is_ok());
        assert_eq!(
            ensure_provider_generation_run_updated(9, 0, "标记 provider 视频生成失败").unwrap_err(),
            "无法标记 provider 视频生成失败：视频 #9 缺少可更新的 generation_run"
        );
    }
}
