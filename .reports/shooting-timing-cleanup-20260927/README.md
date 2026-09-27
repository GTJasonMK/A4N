# 剧本估时移除后的清理与回归

本轮直接修改 `debug`，以 `.dev-runtime/shooting-timing-cleanup-20260927/source-before/` 为审查基线。保留其他会话的改动，未整体提交、合并或推送。

## 清理与修复

- 视频投影以导演约束提供时长和画幅，删除重复时长、片段数量、空风险字段和无人读取的审计结果。投影中的导演约束改为必填，提示词构建直接使用类型，删除 JSON 转换往返。
- 每个导演镜头对应一个片段，删除旧拆段计数、默认时长/画幅回退、额外转场片段生成，以及所有调用方都丢弃结果的解析包装函数。
- 读取已存片段时按导演画幅验证，修复页面默认画幅与导演画幅不同导致规划被误判无效的问题。
- 片段校验保留镜头顺序、人物约束和必需节拍覆盖，并补上节拍顺序校验，拒绝把同一组节拍倒序输出。
- 完成调用的升级回执统一在数据库解码入口验证，覆盖严格指纹命中的加载路径。删除恢复时的第二次查询、上层重复校验和整份回执的重复传递。
- 包导入先验证原始输入哈希和升级回执，再映射 ID、计算新哈希。原本没有指纹的历史记录继续保持无指纹；损坏数据不能通过重算哈希变成可复用调用。
- 旧数据转换只登记实际被引用的剧本、来源和镜头哈希，删除递归过程中的整树复制与无用包装节点哈希。迁移、包导入共用转换和回执计算。
- 删除废弃估秒值的正数、求和校验，保留原始内容哈希及当前剧本合同校验。更新 Mock Provider，固定输出一个导演片段；旧版 CLI 回归在两个阶段均显式配置同一运行快照。

本轮 Rust 改动涉及 35 个文件，新增 554 行、删除 937 行，净减 383 行；按本轮开工副本计算，未计入其他会话对 `context/project/bundle_transfer.rs` 的修改。

关键文件：

- `crates/video/a4n-video-contract/src/contracts.rs`
- `crates/host/a4n-orchestrator/src/video/prompts/json/segment_parser.rs`
- `crates/host/a4n-orchestrator/src/video/prompts/json/cross_validation.rs`
- `crates/platform/a4n-db/src/workflow/llm_checkpoint.rs`
- `crates/platform/a4n-db/src/project/bundle.rs`
- `crates/platform/a4n-domain/src/shooting_script/compat/`
- `scripts/tests/run_shooting_timing_upgrade_cli_e2e.py`

## 实际验证

证据目录：`.dev-runtime/shooting-timing-cleanup-20260927/`。

- **679 项不同的后端测试通过**：domain 25、video-contract 9、migration 3、db 64、orchestrator 578。去重清单在 `distinct-backend-tests.json`；其中 124 项视频流程测试已逐项核对完整执行。
- **13 项前端测试、31 项 Mock Provider 测试通过**。API/生成合同一致性、TypeScript、前端生产构建、核心边界、Sidecar 边界、全仓 Rust 格式和 `git diff --check` 通过。
- **34 项真实 CLI 检查通过**：旧版升级 5、导演时间线 18、异步剧本 11，汇总在 `final-cli-suites.json`。使用本轮源码构建的 Server、CLI、Migration 和 Novel Planning Worker，二进制 SHA-256 在 `bin/build.json`。
- 真实旧版样本保留作者已编辑场景、冻结剧本、媒体和调用证据。升级后，剧本和片段规划各只补生成缺失的一个镜头；重复执行完成的片段规划没有 Provider 调用。
- v5/v6 项目族包及统一项目包导入通过。篡改调用输入哈希或升级回执会明确失败，回滚后项目列表不变。
- 从迁移前备份新建真实用户库的隔离副本并执行本轮迁移。1 条剧本、7 条导演方案、3 个来源快照、78 个工作流、377 条调用断点、7 条任务断点和 2 张图片的数量不变；非升级字段、剧本内容、来源引用、原始请求/响应/用量均保留，6 条完成调用获得升级回执。归档工作流保持原状态。结果在 `user-copy-preservation.json`。

初次失败和超时日志均保留。新增画幅测试先遇到旧夹具的废弃参考图字段，修正夹具后通过；两组数据库测试触及 60 秒限制，按更小分组补齐剩余用例，没有放宽超时。新隔离环境的 CLI 配置、运行快照和媒体目录问题修正后重跑通过。API 检查中的命令排序差异由生成器修正。前端构建仍有既有的 chunk 体积提示。

## 保留的边界与收尾

冻结的 V1 合同、旧模板映射和 v5/v6 导入转换仍由历史迁移和旧项目使用；保留在显式升级入口。独立画布及既有非导演片段仍使用可选的片段导演约束。视频执行 epoch 保持 5。

只读检查确认真实用户库尚未应用 `shooting_script_timing_v2`，没有 V2 剧本或 V3 断点输入，因此继续完善尚未部署的 V3 投影合同。**未手工迁移或修改真实用户库。** 正常启动时由既有迁移入口升级；回退仍需要迁移前备份。

本轮 Server/Worker、Mock Provider 和 Qdrant 已全部停止，检查未发现本轮残留进程。共享 agent target 在独占锁内清理了 6.4 GiB，随后释放构建锁。源码、私有测试证据及备份保留，其他会话的进程与改动未清理。
