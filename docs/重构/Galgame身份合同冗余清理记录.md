# Galgame 身份合同冗余清理记录

## 搜索覆盖范围

本专项从 Galgame 工作区的脚本、管线、资产和导出功能出发，按 BFS 追踪完整生产链路：

```text
GalgameProjectWorkspace
→ GalgameTab / PipelineStatusPanel / MediaAssetIdBinder
→ frontend api/galgame
→ apiInvoke
→ Web authenticated command / Tauri invoke_host_command
→ Server galgame dispatch 与 script/pipeline/assets handlers
→ Runtime galgame_records actions
→ Orchestrator Galgame application
→ collaboration project permission
→ DB galgame/workflow/media/project membership records
```

横向覆盖 17 条活跃命令：脚本读取，管线启动、取消、预览和状态查询，行插入、更新、删除和重排，场景与选项更新，资产规划与生成，媒体绑定、解绑、音频导入及 WebGal 导出。隐式依赖覆盖 command manifest context/schema、生成 Host 命令表、Web Bearer token、Tauri 外层 `user_id`、workflow 任务注册、runtime config、文件导入导出能力和项目 Viewer/Editor 权限。仓库中没有 Galgame 相关 XML DI、反射、动态代理或 AOP；动态入口仅为 Host 命令分派、workflow 和生成资产 Host port，均已纳入闭包。

## 清理前分叉

```text
Web JWT / Tauri invoke_host_command.user_id
→ Server user_id
→ Runtime
→ Orchestrator verify_project_permission

frontend api/galgame
→ 17 个命令再次发送 userId
→ Server handler 不读取该业务字段
→ 宽松外层 schema 继续允许退役身份参数
```

Viewer/Editor 权限始终使用 Host 认证身份；业务参数中的 `userId` 不参与权限、查询、写入、workflow、媒体或导出逻辑。它只是第二身份源，并由宽松 schema 无条件放行。

## 优化记录

1. `get_galgame_script` 删除重复 `userId`。
2. `start_galgame_pipeline` 删除重复 `userId`。
3. `cancel_galgame_pipeline` 删除重复 `userId`。
4. `preview_galgame_pipeline_request` 删除重复 `userId`。
5. `get_galgame_pipeline_status` 删除重复 `userId`。
6. `update_galgame_line` 删除重复 `userId`。
7. `insert_galgame_line` 删除重复 `userId`。
8. `update_galgame_scene` 删除重复 `userId`。
9. `delete_galgame_line` 删除重复 `userId`。
10. `reorder_galgame_lines` 删除重复 `userId`。
11. `update_galgame_choices` 删除重复 `userId`。
12. `plan_galgame_assets` 删除重复 `userId`。
13. `run_galgame_asset_generation` 删除重复 `userId`。
14. `bind_galgame_media` 删除重复 `userId`。
15. `unbind_galgame_media` 删除重复 `userId`。
16. `import_galgame_audio` 删除重复 `userId`。
17. `export_galgame_webgal` 删除重复 `userId`。
18. 删除 `frontend api/galgame` 对 `requireCurrentUserId` 的零剩余依赖。
19. 7 个 input-only schema 改用既有 `strict_typed_input_schema`，不新增 Galgame 专用 helper。
20. 仅供 6 个 Galgame 项目变更命令使用的 `project_typed_input_schema` 改为拒绝额外字段。
21. 新增一项 manifest 回归，一次验证 17 条命令均不声明或接受业务 `userId`。

## 清理后唯一事实标准

```text
用户身份
→ Web：Bearer token 认证出的 user_id
→ Tauri：invoke_host_command 外层 user_id

项目范围
→ projectId 或 input.project_id

权限
→ Orchestrator verify_project_permission
→ Viewer：脚本与状态读取
→ Editor：管线、编辑、资产、媒体与导出
```

未新增旧参数兼容、身份覆盖规则、默认用户、权限回退或 transport 适配器。

## 集中验证

- `npx --no-install tsc --noEmit`：通过。
- `npm run check:web-api-parity`：通过；API 类型、generation schema、TypeScript 输出和 fixtures 均无漂移。
- `cargo test -p a4n-api-contract command_manifest::tests::galgame_commands_use_one_authenticated_identity`：1 项目标测试通过，复用仓库现有 `target/`。
- `rg` 退役字段残留搜索：通过，`frontend api/galgame` 中无 `currentUserId` 或业务 `userId`。
- `git diff --check`（仅本专项文件）：通过。
