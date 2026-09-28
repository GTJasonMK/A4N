# 漫画与 GALGame 工作流

## 漫画项目

### 前置与流水线

漫画项目使用冻结的来源章节正文，独立生成本地改编计划。无需先建立视频或拍摄剧本。工作区分为五步：

`基础素材 → 改编计划 → 分页分镜 → 页面提示词 → 图片生成`

使用 `start_manga_pipeline` 首次运行，`resume_manga_pipeline` 从断点继续；`stop_after=page_planning` 生成独立分页草稿，审阅、保存并采用后，`stop_after=drawing_prompt` 继续页面提示词。默认一键流程到页面提示词为止，图片生成需要独立操作。开始前可用 `preview_start_manga_pipeline_request`，继续前可用 `preview_resume_manga_pipeline_request`，状态用 `get_manga_pipeline_status`。

### 页面与参考图

- 用 `list_manga_page_prompts` 查看页面；用 `update_manga_prompt` 编辑并保存提示词。
- 用 `optimize_manga_prompt` 让 LLM 提出优化，先审阅再保存。
- 用 `bind_manga_reference_image` 绑定角色主立绘、项目图片或其他素材；已有成图时变更绑定要先处理下游冲突。
- 用 `preview_page_image_request` 检查真实生图请求，再通过图片生成能力生成/更新页面。
- `delete_manga_prompt` 或 `delete_manga_by_chapter` 会删除本章提示词和产物，执行前确认。

先选择章节，再按工作区阶段推进。补齐缺失立绘属于独立图片任务，需先确认图片能力和主图约束。

## 独立计划与可选视频参考

漫画和 Galgame 共用创作文档命令：`get_media_authoring_workspace` 读取草稿、采用版本与下游过期状态；`generate_media_adaptation_plan` 生成改编草稿；`preview_media_adaptation_plan_request` 预览；`save_media_authoring_document` 保存；`adopt_media_authoring_document` 明确采用。修改必须携带当前 `record_id` 与 `expected_revision`，遇到版本冲突先重读。

`list_video_adaptation_references` 只列出同原作可读取的视频剧本。引用保存固定版本和内容；视频更新只显示差异，重新生成并采用才更新参考，删除视频也不删除已经捕获的参考。原文/采用计划变更只将下游标为需更新，保留人工文本、页面与媒体。

## GALGame 项目

### 七阶段工作区

Galgame 从来源章节独立改编，场景和演出引用本项目的改编单元与节拍：

`基础素材 → 改编计划 → 分支设计 → 剧本编辑 → 资产计划 → 资产生成 → 导出`

基础素材阶段复用人物立绘及场景、道具素材面板，可先生成或上传参考图；依赖剧本的 CG 等内容仍在资产计划之后生成。素材准备不会强制阻塞纯文本改编。

用 `start_galgame_pipeline` 启动，`get_galgame_pipeline_status` 查询，`preview_galgame_pipeline_request` 预览，`cancel_galgame_pipeline` 取消。`stop_after=branch_design` 保存分支草稿并返回空剧本结果；采用分支计划后，`stop_after=script` 继续剧本与资产计划。一键流程不会自动生成图片、声音或视频。继续或重跑时选择明确起始阶段；不要用空占位结果跳过缺失 checkpoint。

### 脚本编辑约束

- 场次可编辑标题、地点、时间、背景需求、类型和下一场，并绑定/解绑背景。
- 行类型包括旁白、对白、独白、演出和选择；对白/独白要绑定角色时先确认档案。
- 编辑文本、表情、姿势、站位、转场和事件 CG 后保存；行顺序变化用专用 reorder 操作。
- 演出明确区分原文、改编、新增和历史内容。新增内容可以引用本地节拍，但不能伪造原文锚点；原文/改编内容必须具有可验证的来源。
- 选择行必须保留 2–3 个选项，每个选项指向合法目标场；支线必须回收到主线。保存或导出前让服务端校验，不要手工绕过断链提示。

当前命令族是 `get_galgame_script`、`update_galgame_scene`、`update_galgame_line`、`insert_galgame_line`、`delete_galgame_line`、`reorder_galgame_lines`、`update_galgame_choices`。

### 资产与导出

1. 用 `plan_galgame_assets` 规划背景、角色差分和事件 CG。
2. 用 `run_galgame_asset_generation` 批量或单项生成；先检查 `required_capabilities` 和角色表情/姿势参数。
3. 用 `bind_galgame_media` / `unbind_galgame_media` 绑定背景、CG、BGM、SFX；音频用 `import_galgame_audio`。
4. 检查缺失媒体、来源快照和场/行绑定后，用 `export_galgame_webgal` 导出 WebGAL 工程。

生成资产、绑定媒体、删除行/场和导出失败都会显式报告合同错误；不要把“有 generation spec”当成“已有可用媒体”。
