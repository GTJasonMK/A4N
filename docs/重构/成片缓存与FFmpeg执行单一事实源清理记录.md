# 成片缓存与 FFmpeg 执行单一事实源清理记录

日期：2026-08-16

## BFS 覆盖范围

本批从章节成片、画布成片、画布抽帧、视频裁剪和成片后处理五项生产功能出发，按层追踪：

```text
Frontend VideoAssemblyWorkspace / VideoPostProcessWorkspace / AssembleVideoNode
→ generated Host API / command manifest
→ Server 字符串 dispatch / Tauri invoke_host_command
→ Runtime video_records action
→ Orchestrator 权限、项目/章节、workflow 与媒体记录校验
→ assembly request context / Canvas assembly plan
→ cache manifest、音频探测、FFmpeg/FFprobe 实际命令
→ 成片文件、manifest、DB 记录与失败清理
→ AppState 启动过期扫描
→ cleanup outbox
```

横向比较了章节预览、成片推送、画布成片三条 `run_ffmpeg_assemble_videos` 调用链，以及抽帧、裁剪、后处理的命令启动与外层持久化补偿。搜索覆盖 `a4n-media`、`a4n-runtime`、`a4n-orchestrator`、`a4n-api-contract`、Server/Tauri Host、前端入口、进程/单元测试、文档和常见 XML/YAML/JSON/TOML 配置入口。

静态搜索未发现反射、动态代理、AOP、DI 容器、XML 或路由配置中的隐藏 FFmpeg 预检或成片缓存实现；Tauri 仍通过通用 `invoke_host_command` 进入与 Server 相同的 Runtime action。

## 已执行清理

1. 过期成片缓存路径改用 Core `data_dir_relative_file_path`，删除 Runtime 私有 `relative_path` 实现。
2. 章节成片失败后只保留 `persist_assembly_preview_cache` 的同步双文件清理，删除清理成功后再次为已不存在文件登记 outbox 的第二路径。
3. 删除上述第二路径制造的必然“cleanup 路径不存在”二次错误。
4. 三条成片渲染调用均已由 `persist_assembly_preview_cache` 或 `persist_created_media_file` 负责失败清理，删除共享 renderer 内部的重复文件删除。
5. 后处理渲染由 `persist_assembly_record_output` 唯一负责失败清理，删除 renderer 内第二次删除同一输出文件的逻辑。
6. 画布抽帧删除执行实际 FFmpeg 命令前的 `ffmpeg -version` 预检。
7. 视频裁剪删除同一预检。
8. 章节预览、成片推送和画布成片经共享 renderer 删除同一预检。
9. 成片后处理删除同一预检。
10. Runtime 删除失去全部调用者的 `ensure_ffmpeg` 及两层 re-export。
11. `a4n-media` 删除只服务上述预检的 `FfmpegAvailabilityError` 与 `check_ffmpeg_available` 公共 API；实际媒体命令自身的 typed `Unavailable/Failed` 成为唯一执行事实。
12. FFmpeg smoke 测试直接用标准库检查 `ffmpeg` 与 `ffprobe` 两个真实依赖，删除对生产预检 API 的测试占用。
13. 画布成片删除 `|_| Ok(false)` 的静音分叉，改用章节成片相同的 canonical manifest builder 和真实音频探测。
14. 生产 imports prelude 不再暴露可注入音频探测器；注入入口仅保留给现有测试夹具，生产调用全部进入唯一真实探测路径。

未新增默认值、兼容、回退、占位逻辑或第二套 FFmpeg/缓存实现。

## 保留的有效边界

- `projects/` 或项目 `videos/` 尚不存在表示当前没有可扫描缓存，`NotFound → 空集合` 属于正常首次运行状态，继续保留。
- 过期 manifest 及其现存成片文件仍由启动扫描登记 cleanup outbox；这与单次渲染失败时立即删除当前输出是两个不同生命周期。
- FFmpeg/FFprobe 的实际命令错误继续显式返回；本批只删除命令前重复启动另一个探测进程的预检。
- `build_assembly_cache_manifest_with_audio_probe` 继续服务无需启动 FFprobe 的确定性测试夹具，不进入生产调用路径。

## 验证状态

遵照资源约束，本批仅使用源码阅读、`rg`、普通 diff 和 `apply_patch`；未运行 Cargo、npm、测试、构建、lint、fmt、数据库、迁移、浏览器、`git diff --check` 或任何验证脚本。集中验证须经用户明确同意，并复用仓库现有 `target/`。
