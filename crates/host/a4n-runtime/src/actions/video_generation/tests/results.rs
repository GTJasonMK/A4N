//! Host 仅保留烟雾：纯规则以 `a4n-video` 为准。

use a4n_video::ensure_provider_generation_run_updated;

#[test]
fn provider_generation_run_update_smoke_matches_orch() {
    ensure_provider_generation_run_updated(42, 1, "完成 provider 视频生成")
        .expect("updated generation run should satisfy provider lifecycle");

    let err = ensure_provider_generation_run_updated(42, 0, "完成 provider 视频生成")
        .expect_err("missing generation run target must be visible");
    assert_eq!(
        err,
        "完成 provider 视频生成失败：视频 #42 缺少可更新的 generation_run"
    );
}
