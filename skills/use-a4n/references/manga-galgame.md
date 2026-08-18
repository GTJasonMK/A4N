# 漫画与 GALGame 工作流

## 漫画项目

### 前置与流水线

漫画项目必须有可用的来源章节正文；正文正在生成或为空时先回到写作桌。准备角色主立绘和场景/物体素材后再开始。流水线大致为：

`角色绑定 → 对白/想法/旁白提取 → 场景与视觉锚点 → 关键物品 → 章节节奏 → 分页规划 → 整页提示词 → 成图`

使用 `start_manga_pipeline` 首次运行，`resume_manga_pipeline` 从断点继续；开始前可用 `preview_start_manga_pipeline_request`，继续前可用 `preview_resume_manga_pipeline_request`，状态用 `get_manga_pipeline_status`。正文或来源快照变化后，从受影响阶段更新或重跑，不要直接复用过期页。

### 页面与参考图

- 用 `list_manga_page_prompts` 查看页面；用 `update_manga_prompt` 编辑并保存提示词。
- 用 `optimize_manga_prompt` 让 LLM 提出优化，先审阅再保存。
- 用 `bind_manga_reference_image` 绑定角色主立绘、项目图片或其他素材；已有成图时变更绑定要先处理下游冲突。
- 用 `preview_page_image_request` 检查真实生图请求，再通过图片生成能力生成/更新页面。
- `delete_manga_prompt` 或 `delete_manga_by_chapter` 会删除本章提示词和产物，执行前确认。

UI 路径是“同步来源 → 选择章节 → 生成本章漫画 → 查看阶段/请求 → 页面提示词与参考图 → 生成/更新图片”。补齐缺失立绘属于独立图片任务，需先确认图片能力和主图约束。

## GALGame 项目

### 七阶段剧本管线

GALGame 从来源章节和拍摄剧本生成结构化脚本，典型阶段为：

`directing_projection → branch_design → script_adaptation → character_staging → visual_planning → persist_script → asset_generation`

用 `start_galgame_pipeline` 启动，`get_galgame_pipeline_status` 查询，`preview_galgame_pipeline_request` 预览，`cancel_galgame_pipeline` 取消。继续或重跑时选择明确起始阶段；不要用空占位结果跳过缺失 checkpoint。

### 脚本编辑约束

- 场次可编辑标题、地点、时间、背景需求、类型和下一场，并绑定/解绑背景。
- 行类型包括旁白、对白、独白、演出和选择；对白/独白要绑定角色时先确认档案。
- 编辑文本、表情、姿势、站位、转场和事件 CG 后保存；行顺序变化用专用 reorder 操作。
- 选择行必须保留 2–3 个选项，每个选项指向合法目标场；支线必须回收到主线。保存或导出前让服务端校验，不要手工绕过断链提示。

当前命令族是 `get_galgame_script`、`update_galgame_scene`、`update_galgame_line`、`insert_galgame_line`、`delete_galgame_line`、`reorder_galgame_lines`、`update_galgame_choices`。

### 资产与导出

1. 用 `plan_galgame_assets` 规划背景、角色差分和事件 CG。
2. 用 `run_galgame_asset_generation` 批量或单项生成；先检查 `required_capabilities` 和角色表情/姿势参数。
3. 用 `bind_galgame_media` / `unbind_galgame_media` 绑定背景、CG、BGM、SFX；音频用 `import_galgame_audio`。
4. 检查缺失媒体、来源快照和场/行绑定后，用 `export_galgame_webgal` 导出 WebGAL 工程。

生成资产、绑定媒体、删除行/场和导出失败都会显式报告合同错误；不要把“有 generation spec”当成“已有可用媒体”。
