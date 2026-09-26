# 完成记录

接续 Claude Code `b5282908-0b99-4786-ab5d-b25b7b21d906`。遵照用户明确要求，全部业务修改位于 `/home/fufu/Code/A4N/work/debug`，未新开分支，未整体提交或回滚并行工作。开始基线 `c8deec1a`。

完整实现与覆盖以 [README.md](README.md) 为准，媒体发送边界见 [media.md](media.md)。已完成来源表示/裁剪、布局来源选择 v2、同阶段规则与跨角色共享章节的前缀排序、Agent 运行状态后移、重复载荷删除、负面约束透传、镜头参与者范围和显式续接来源上下文。

验证结果保存在 [validation.json](validation.json)：699 项不同 Rust 用例、24 项前端、27 项 Mock Provider、三种 Chromium 请求、13 个文本 CLI 场景、17 项导演/媒体 CLI 检查、8 项四协议缓存检查通过。完整运行模式构建、类型、合同、全库格式和 diff 检查通过。

失败记录没有记为通过：早期的 schema 初始化、minimal 模式功能包禁用、旧来源身份夹具、整组测试超时、缓存测试未显式启用选项、错误 API 基址和 1 秒视频样本均已定位并按实际合同修正。最终 CLI 使用 full runtime、独立 PostgreSQL 和独立 Qdrant；没有篡改 package 安装状态或跳过权限/来源门禁。

私有材料、日志、环境、二进制 hash 和失败过程记录在 `data/prompt-context-audit-20260926/`，不提交。历史样本在 `legacy/`；修改前副本在 `before/`，保留当时其他会话的 WIP。两个晚补副本（视频 shooting_script_projection.rs、video/prompts/json.rs）通过逆向去掉本任务明确补丁恢复，不应把整个当前 git diff 都归于本任务。

共享锁按用户“你直接可以吗”的授权协调：仅在当前编译/测试批次结束后释放空闲持锁 shell，未删除锁文件，也未杀掉活动 cargo/rustc 或测试进程。代码保留为已验证的未提交工作；协调表保留记录，供后续整合。

收尾：停止本任务的测试服务，保留数据库/请求样本供复核；共享构建产物按锁内清理。实际远端模型质量与缓存命中率未做 A/B，Mock usage 明确为合成数据。

本次 cargo clean 返回 `Removed 0 files`；共享 target 随后已由其他会话继续使用，未删除其后续生成的产物。本任务服务的 19190/19193/19194/19196/19197/19198 端口均已关闭，独立 PostgreSQL 容器已停止。
