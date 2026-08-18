# 设置、协作与系统维护

## 模型与能力配置

按需配置，不要为了“全开”保存不必要的密钥：

1. 先配置并测试 LLM；文本生成、分析、提示词优化和多数结构化管线依赖它。
2. 需要资料检索或章节上下文时配置 Embedding，并同步 RAG 索引。
3. 需要立绘、场景、物体、漫画页或事件 CG 时配置图片服务。
4. 需要片段或成片时配置视频服务，确认宽高、FPS、并发、时长模式和 provider 支持的输入传输方式。

UI 中统一使用“新建/保存 → 检测模型列表 → 测试连接 → 设为默认”。CLI 先用 `capabilities describe` 查看命令的 `required_capabilities`，不要靠失败请求猜配置是否可用。

不要从统一 UI 按钮猜 CLI 名称；当前创建入口并不完全同名。版本变化时先用 `commands config` 搜索候选，再对选中的命令执行 `capabilities describe`。常用入口是：

- LLM：`create_llm_config`、`update_llm_config`、`detect_llm_config_models`、`test_llm_config`、`set_default_llm_config`、`list_llm_config`。
- Embedding：`create_embedding_config`、`update_embedding_config`、`test_embedding_config`、`set_default_embedding_config`、`list_embedding_config`。
- 图片：`add_image_config`、`update_image_config`、`detect_image_config_models`、`test_image_config`、`set_default_image_config`、`list_image_configs`。
- 视频：`add_video_config`、`update_video_config`、`detect_video_config_models`、`set_default_video_config`、`list_video_configs`；当前没有独立的视频配置测试命令，用模型探测和目标视频能力的只读预览验证配置。

服务端共享配置和个人配置的权限不同，遇到 forbidden 先检查账号角色。

## 提示词、主题和配置备份

- 提示词：用 `prompt_list` / `prompt_list_categories` 找到模板，先 `prompt_render` 或 `prompt_validate_variables`，再 `prompt_update`；项目级覆盖用 `prompt_set_project_override`，恢复全局用 `prompt_remove_project_override`。
- 版本：修改前查看 `prompt_version_history`，比较用 `prompt_version_diff`；不要把项目覆盖误当成全局模板。
- 主题：用 `list_themes`、`activate_theme`、`create_theme`、`update_theme`、`export_theme`/`import_theme`；删除主题前确认当前激活主题不会丢失。
- 配置备份：`export_all_configs_json` / `import_all_configs`。导出的 JSON 可能包含敏感配置，使用受限文件权限，不要提交或发送。

## 队列、日志与运行诊断

- UI 的“设置 → 请求队列”或 `get_queue_stats` 查看等待/执行数量、优先级和活跃工作流。
- 用 `list_pending_tasks`、`get_task_history`、`get_workflow_run_detail`、`get_workflow_progress_events` 追踪具体运行；取消前先核对项目、任务和 attempt。
- 日志使用 `read_recent_logs`、`get_log_dir`、`set_log_level`；生成调用统计使用 `get_generation_call_analytics`。日志可能包含业务上下文，分享前先脱敏。
- `doctor --strict` 只适合诊断本地部署的环境 warning；合同不兼容时先升级/对齐 CLI 与 Server，不要绕过 manifest 检查。

## 协作与权限

项目角色从高到低通常是：拥有者、管理员、编辑者、只读成员。邀请、改角色、移除成员和离开项目都要确认目标用户与项目：

- UI：项目详情 → 协作成员 → 邀请/调整/移除。
- CLI/API：`invite_project_member`、`update_project_member_role`、`remove_project_member`、`list_project_members`、`list_project_invites`、`accept_project_invite`、`decline_project_invite`、`revoke_project_invite`、`leave_project`。

只读成员不应尝试通过 CLI 绕过 UI 禁用按钮；服务端权限检查才是最终边界。

## 账户、会话与管理员

- 个人账户优先使用安全语义命令：`auth update-profile`、`auth change-password --passwords-stdin`、`auth sessions`、`auth revoke-session SESSION_ID --yes`、`auth refresh` 和 `auth logout`。
- 管理员用 `get_storage_paths` 查询服务端存储位置，用 `list_orphan_project_data_dirs` 扫描孤儿目录，并只在展示精确目录后用 `cleanup_orphan_project_data_dirs` 清理。
- 管理员先检查 `get_admin_overview` 和当前角色；用户/邀请码使用 `create_user`、`update_user_role`、`disable_user`、`create_invite_code`、`list_invite_codes`，全局项目/任务/用量使用 `list_all_projects`、`cancel_admin_task`、`get_token_summary`、`get_usage_trends`。
- 禁用用户、取消任务、清理目录和修改共享默认配置会影响其他用户，必须展示目标与影响并取得明确授权。

## 依赖和危险操作

正文、角色/场景档案、提示词和源快照是下游素材的上游事实。修改或删除前查看依赖图（`get_project_asset_dependency_graph`、`get_project_asset_dependency_graph_node_detail`）和过期标记。

删除项目、重置生成数据、回滚阶段/快照、清理孤儿目录、取消任务、改变共享配置、移除成员和应用批量 patch 都需要明确授权；先预览、再确认、最后执行。配置密钥、访问 token 和 OAuth code 永远不写入命令参数、日志、普通脚本/参数文件或导出包。
