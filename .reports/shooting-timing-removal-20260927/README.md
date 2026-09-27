# 拍摄剧本估时移除

直接修改 `debug`，保留工作区其他会话的改动，未整体提交或合并。

已从正常业务合同、校验、编辑器和模型提示词中删除节拍 `estimated_seconds`、镜头 `estimated_natural_seconds`、视频投影 `natural_seconds`、估时合计与由估时推导的动作负载。新增、删除节拍不再分配或转移秒数。镜头时长取导演时间轴；实际视频时长仍从媒体文件读取。节拍内容、稳定 ID、顺序、来源、必需覆盖和有效时间轴校验保留。

合同升级为 `chapter_shooting_script_v2`、`video_execution_projection_v3` 和 `project_family_v7`；视频执行 epoch 保持 5。Novel Planning Worker 协议升为 v3，避免与旧 Worker 混用。

`shooting_script_timing_v2` 迁移升级现存剧本、冻结快照、素材引用、可恢复工作流及完成调用。旧哈希先验证，再精确替换匹配的引用；原本过期的引用不会被标成有效。原始模型请求、响应和用量保存在升级回执中。损坏哈希或缺少完成调用指纹会明确失败，事务回滚。取消且不可恢复的工作流状态、进度和调用断点保持原状。

旧 v5/v6 项目族包和统一项目包共用升级入口。未确认的导演草稿保持草稿，后续仍走原有包合同校验。跨项目导入沿用既有运行作用域规则：无法映射的任务只保留历史，不能自动接管；本次没有放宽该限制。

关键代码：

- `crates/platform/a4n-domain/src/shooting_script/contracts.rs`：当前剧本合同。
- `crates/platform/a4n-domain/src/shooting_script/compat/`：冻结的旧合同、哈希映射及断点升级。
- `crates/platform/a4n-migration/src/migrations/shooting_script_timing_v2.rs`：事务迁移。
- `crates/host/a4n-orchestrator/src/context/workflow/timing_upgrade.rs`：只复用有升级回执且请求内容仍匹配的旧调用。
- `crates/host/a4n-orchestrator/src/project/bundle/shooting_upgrade.rs`：两种包入口共用的转换。
- `frontend/src/pages/novel-detail/shooting-script/StructuredScriptEditor.tsx`：删除估秒控件，显示导演镜头时长。
- `scripts/tests/run_shooting_timing_upgrade_cli_e2e.py`：真实旧版生成、迁移、新版续跑及旧包导入回归。

验证证据位于 `.dev-runtime/shooting-timing-removal-20260926/`：

- 543 项不同的后端定向测试通过，覆盖合同、规划、包传输、PostgreSQL 迁移与数据库导入。计数去除了重复执行；清单为 `distinct-backend-tests.json`。
- 31 项 Mock Provider 测试、3 个前端文件的 11 项定向测试通过。
- API/生成合同一致性、TypeScript 与生产前端构建、核心与 Sidecar 边界检查通过；前端构建仍有既有的大 chunk 提示。
- 真实 Chromium 验证通过：1440/1024/390px 无横向溢出，键盘增删节拍及保存请求均不含估时字段。截图和测量在 `browser-r4/`，另已完成独立截图复核。
- `final-upgrade-cli.json`：旧版实际保存半成品剧本和片段规划后，最终程序分别只调用缺失的一个镜头；重复已完成片段规划没有 Provider 调用。v5/v6 两种包入口导入、原始调用证据与媒体保留通过。
- `final-director-cli.json` 的 18 项检查、`final-shooting-cli.json` 的 11 项检查通过；连同升级回归的 4 项，共 33 项真实 CLI 检查。包括导演时间、参考图、实际视频时长、编辑保留、停止/续跑和派生项目来源同步。
- `user-copy-preservation.json`：真实用户数据库的独立副本迁移通过；1 条剧本、7 条导演方案、3 个来源快照、78 个工作流、377 条调用断点、7 条任务断点、2 张图片的数量及非估时字段保留。原始响应与用量未改写，升级了 6 条完成调用。工作流 81 在备份时已是 cancelled/can_resume=false，保持该状态。

测试过程中的失败均保留：首轮 Qdrant 使用了不支持的配置后缀；旧包首次测试错误地假定跨项目作用域都能接管；恢复测试数据库后，上一轮媒体文件与复用的 ID 冲突；导演回归首次使用了默认 1 秒视频夹具，实际时长检查拒绝把它当成 6 秒。分别修正测试配置、按现有导入合同收敛检查、隔离上一轮媒体，并使用经 ffprobe 确认的 6 秒夹具，没有放宽生产校验。

真实用户库未手工迁移。已保存权限为 0600 的迁移前备份 `private-user-before.dump`；正常启动入口会先构建新程序再运行迁移，避免新 schema 与旧程序混用。回退必须恢复迁移前备份，不能从新合同反推已经删除的估秒值。

最终 Server、CLI、Migration 与 Novel Planning Worker 构建通过，程序摘要在 `bin/build.json`；最终生成合同检查、全仓 rustfmt 和 `git diff --check` 通过。本次 Server/Worker、Mock Provider、Qdrant 已关闭；共享 agent target 在独占锁内清理了 4.9 GiB，构建锁已释放。测试数据和备份保留供审计，未停止其他会话的服务。
