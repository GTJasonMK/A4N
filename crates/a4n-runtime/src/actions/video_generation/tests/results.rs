use super::*;

#[test]
fn provider_generation_run_update_requires_at_least_one_running_run() {
    ensure_provider_generation_run_updated(42, 1, "完成 provider 视频生成")
        .expect("updated generation run should satisfy provider lifecycle");

    let err = ensure_provider_generation_run_updated(42, 0, "完成 provider 视频生成")
        .expect_err("missing generation run target must be visible");
    assert_eq!(
        err,
        "完成 provider 视频生成失败：视频 #42 缺少可更新的 generation_run"
    );
}
