# 视频项目与自由画布

## 来源视频项目前置条件

来源视频以文本项目和来源章节为基础。先保存正文，再创建 `derived_kind=video` 的派生项目；导演编排和拍摄剧本在这个项目中按章生成，其他视频的结果不会自动复制过来。每个阶段都可能产生可复用草稿；不要在上游变化后直接采用旧候选。

## 视频流水线

按工作区七步处理：**基础素材 → 导演编排 → 拍摄剧本 → 片段规划 → 片段生成 → 成片合成 → 成片后处理**。基础素材提供人物立绘、场景和道具的生成及上传入口；片段和成片仍在下游生成。

### 1. 基础素材

筛选角色、场景、物体和素材状态；批量补缺或更新过期素材。可上传并设为主素材，也可复制/预览提示词。修改或删除主素材前检查引用它的片段和参考媒体。

### 2–3. 导演编排与拍摄剧本

所有命令使用当前视频项目的 `project_id` 和原文章节的 `source_chapter_id`。通过 `get_chapter_visual_plan` 读取方案和可选场景，`save_chapter_visual_plan_draft` 保存，`confirm_chapter_visual_plan` 采用。随后用 `start_chapter_shooting_script_generation` 生成，读取用 `get_chapter_shooting_script`，结构化修改用 `edit_chapter_shooting_script`；只观察实际返回的非终态任务。

### 4. 片段规划

按拍摄剧本场景创建或重建片段计划；未完成的计划优先继续，不要重复创建。规划配置包含共享风格、氛围和细节要求。删除规划会影响下游提示词和视频，必须确认。

### 5. 片段生成

为每个片段选择执行方案（独立首尾帧、首帧续接、分镜板、参考图/媒体或外部 Chat）。准备首帧、尾帧、续接首帧和参考媒体后：

1. 生成并检查最终视频提示词（`preview_chapter_video_request`、`update_chapter_video_prompt`、`optimize_chapter_video_prompt`）。
2. 生成、轮询或导入视频（`start_video_generation`、`poll_video_job`、`upload_chapter_video`）。
3. 刷新候选，检查时长/来源，必要时 `trim_video`，最后 `select_segment_video` 作为合成输入。

切换执行方案可能使提示词、候选视频和采用状态失效；先保存或记录当前选择。
`select_segment_video` 会改变采用状态，按 descriptor 的 destructive 规则取得明确授权。

### 6. 成片合成

确认片段顺序、候选时长和过场后，使用 `assemble_chapter_videos` 生成预览，使用 `load_cached_assembled_chapter_video_preview` 查看已有预览。预览确认后才能按批次 `push_assembled_chapter_video` 进入后处理；上一批未同意/拒绝时不要推送下一批。

### 7. 成片后处理

后处理是批次审批和版本编辑：先查看批次状态，再用 `update_assembly_push_status` 同意或拒绝。批次审批以及时间线单点插入、区间替换、删除（`upload_assembly_insert_video`、`apply_assembly_postprocess_edit` 等）都按 descriptor 的 destructive 规则确认授权；编辑会创建新版本。完成后下载对应版本，不要覆盖旧成片。

## 当前 CLI 命令族

优先用 `commands video` 找候选，再用 `capabilities describe` 取得单个命令参数。常见命令族包括：

`list_chapter_video_prompts`、`load_video_segment_plan_draft`、`update_video_segment_plan_draft_recipe`、`preview_reference_media_video_request`、`upload_reference_media_video`、`start_video_generation`、`poll_video_job`、`select_segment_video`、`trim_video`、`assemble_chapter_videos`、`push_assembled_chapter_video`、`update_assembly_push_status`、`apply_assembly_postprocess_edit`。

## 自由视频画布

画布适合没有固定小说来源的节点式视频工作：

1. 用 `create_freeform_video_project` 创建项目。
2. 用 `get_canvas_node_contracts` 查看节点输入/输出和所需能力；不要凭节点名称猜参数。
3. 用 `get_canvas_graph`、`save_canvas_graph` 读取/保存节点、连线、参数和位置；媒体用 `import_canvas_media` 或文本资产命令导入。
4. 先 `preview_canvas_node_request`，再 `run_canvas_node` 或 `run_canvas_graph`。运行完整画布会按依赖顺序执行，运行单节点只验证局部链路。
5. 运行记录出现缺失媒体、失败节点或非终态时，修复上游后再恢复；不要把局部完成当成成片完成。

UI 节点类别是输入（文本/图片/视频/音频）、操作（模板、LLM、图片/视频生成、抽帧、裁剪、组装）、探针和输出。保存实际返回的 task/workflow ID，只观察非终态句柄；同步返回的合成或预览结果直接消费，不要额外 watch。
