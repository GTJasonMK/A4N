# 导入、项目包与来源同步

## 导入已有文本

1. 创建或选择文本项目。
2. 用 `import_project_text_upload` 上传 TXT、Markdown、EPUB 或 DOCX；文件上限为 100 MB，参数中的 `file_name` 必须带受支持扩展名，二进制通过 `--set-base64` 注入。
3. 用 `list_import_queue` 查看解析审核项。逐项 `accept_import_review` 或批量 `batch_accept_reviews`；拒绝和删除审核记录属于有影响操作，先确认目标。
4. 运行 `run_import_project_analysis`，需要进度时查询 `get_project_import_analysis_progress`。分析结果返回后再进入结构反推。
5. 反推前用 `preview_reverse_engineer_request`。统一 `reverse_engineer` 的 `target` 取 `blueprint`、`outline` 或 `part_outline`；它通常等待模型并返回 proposal/warnings 和 `workflow_run_id`。先审阅候选，再用 `confirm_reverse_engineering_candidate` 写入。

UI 对应路径是：**导入分析 → 选择文件 → 导入审核 → 接受章节 → 项目分析 → 结构反推预览 → 应用结果**。预览不写库，应用会改变蓝图或大纲。

## 项目包导入导出

- 根项目族：`export_project_family_data` / `import_project_family_upload`。
- 单个派生项目：`export_derived_project_data`。
- 视频项目包：`export_video_studio_project_data` / `import_video_studio_project_upload`。
- 自由画布媒体：`import_canvas_media`；画布结构使用 canvas 命令保存/导出。

导出响应中的 Base64 用 `--save-base64` 原子写入；导入包用 `--set-base64`。默认拒绝覆盖已有文件。导入后检查项目 ID、来源快照、媒体绑定和派生章节范围，不要假设包内旧 ID 仍适用。

## 创建派生项目

漫画、来源视频和 GALGame 需要根文本项目及来源章节范围。先确认正文/拍摄剧本已保存，再用 `create_derived_project` 指定 `derived_kind` 和范围；自由视频画布使用 `create_freeform_video_project`，不要求小说来源。

## 来源同步

派生项目源文本变化时按以下顺序操作：

1. `preview_source_sync` 或 UI 的“同步来源 → 检查更新”，只读取差异、冲突和严重级别。
2. 无冲突项才使用 `apply_source_sync_decisions`；冲突项明确选择保留本地或按最新来源重生成。
3. 重新检查后再应用；应用前确认受影响的提示词、图片、视频、剧本和媒体绑定。
4. 不再需要跟随源项目时使用 `detach_derived_project`；分离后不会继续自动同步。

源正文、拍摄剧本、档案或提示词变化可能使下游产物标记为过期。不要用“保留旧产物”掩盖来源变化，除非用户明确确认旧版本仍然有效。

## 安全边界

项目删除、包导入覆盖、结构反推确认、来源同步应用、分离来源、重置生成数据和删除下游资产都要先展示影响并获得确认。失败或中断后先读 task/workflow 状态，再重试；不要重复提交未知幂等性的写操作。
