# 文本创作与资料工作流

## 项目阶段

文本项目通常沿着以下阶段推进：

`draft → blueprint_ready → part_outlines_ready → chapter_outlines_ready → writing → completed`

不要直接伪造阶段字段。用项目阶段页面或 `forward_project_status` 推进；回退前先 `preview_rollback`，确认影响后再 `rollback_project_status`。系统会根据蓝图、大纲和正文是否存在拒绝非法推进。

## AI 辅助创作

1. 创建文本项目，选择一种项目类型（`novel_long`、`novel_short`、`script` 或 `prose`）。
2. 在灵感对话中补足标题、题材、角色、冲突和目标读者；CLI 使用 `start_inspiration_generation`，先用 `preview_inspiration_request` 检查请求。
3. 生成蓝图并确认：`start_blueprint_generation`、`start_blueprint_refinement`、`get_blueprint`、`save_blueprint`。需要版本回退时先比较 `diff_blueprint_versions`，再 `restore_blueprint_version`。
4. 生成分部和章节大纲：使用 `start_outline_generation` 或当前 outline descriptor；保存/修改前检查大纲范围和顺序。
5. 生成或导入正文：`start_chapter_generation` 返回 generation task；手工文本使用 `chapter import`/`chapter update`。等待 task 终态后再把正文当作后续输入。
6. 进入写作桌，先保存正文，再运行章节分析和角色/场景同步。影视改编在视频派生项目中进行。

模型请求预览只读，不会写入资料；生成结果提交、确认、恢复和批量应用可能改变项目结构，先向用户展示候选和警告。

## 章节分析与档案

- 章节分析：`generate_chapter_analysis`、`get_chapter_outlines_pacing` 及对应 preview 命令；前序分析未完成或正文为空时先修复前置条件。
- 角色：用 `discover_character_profiles` 发现候选，再用 `sync_all_character_profiles` 或 `sync_character_from_chapters` 同步；冲突先 `check_character_conflict`，不要静默覆盖档案。
- 手工档案：角色使用 `create_profile` / `update_character_profile`，物体使用 `create_object_profile` / `update_object_profile`，场景使用 `create_scene_profile` / `update_scene_profile`；章节提取场景使用 `sync_scene_profiles`。
- 角色历史：用快照、属性事件、行为证据和 `rollback_to_chapter` 保留可追溯变化；删除候选必须显式确认。
- 视觉素材：先配置图片能力，再使用 `start_portrait_generation`、`upload_portrait`、`set_primary_portrait` 或对象/场景素材图命令。正文或档案变化后检查素材是否过期。

## 世界观、关系与事件脉络

- 世界观是蓝图的一部分：先 `get_blueprint`，再用 `update_world_setting` 保存结构化对象、JSON 或文本表示；服务端警告必须回显，不能把格式失败当成功。
- 角色关系来自角色档案的初始关系和后续属性事件。关系图节点位置是浏览器本地布局，不属于 CLI；数据修改使用当前 profile descriptor，并保留时间线已开始后不可改写初始关系的约束。
- 事件脉络综合章节分析和伏笔记录。用 `list_foreshadowing` 查询，`create_foreshadowing` / `update_foreshadowing` 维护；`delete_foreshadowing` 是 destructive，删除前确认章节和回收状态。

## 拍摄剧本

导演编排和拍摄剧本属于视频派生项目，按章节独立管理。先创建或选择视频项目，再进入其前两步；命令中的 `project_id` 必须使用该视频项目 ID。原作写作桌保留跳转入口。漫画/Galgame 独立改编，可选引用视频的固定版本。具体操作见 [video-canvas.md](video-canvas.md)。

## 写作优化

正文优化是“提案 → 审阅 → 应用”，不是直接覆盖：

1. 创建分析或聊天会话（`create_optimization_analysis_session` / `create_optimization_chat_session`）。
2. 先预览请求，再生成候选 patch。
3. 逐条接受/拒绝；只有确认后的 `apply_optimization_patches` 才写回正文。
4. 遇到正文已变、定位不唯一或 patch 重叠时，重新生成提案，不要强行应用。

## RAG 与资料检索

先在设置中配置并测试 Embedding，再同步资料索引（`rag_ingest_all` 或页面的“同步资料索引”）。写作桌的资料检索使用 `rag_enhanced_search`；索引过期时先重新 ingest，不要把旧检索结果当作当前正文事实。

## 图像、导出与交付

章节图片和项目图库使用 asset 能力：`list_images_by_chapter` / `list_images_by_project` 查询，`preview_batch_images_request` 后用 `start_image_generation` 生成，或用 `upload_image` 导入；元数据修改用 `update_image`。`delete_image` 会解除下游绑定，执行前查看依赖并确认。文本交付使用 `export_txt_data`、`export_markdown_data`、`export_epub_data`、`export_pdf_data` 或结构 JSON 导出。项目族和派生项目包见 [import-sync.md](import-sync.md)。
