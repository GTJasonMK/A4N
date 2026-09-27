# 全仓测试及相关冗余代码清理（本轮完成）

用户明确要求全仓清理，继续直接修改 `debug`，保留其他会话的在途改动。本轮不以阶段 E2E 替代所有单测，也不以接口数量宣称业务全覆盖。

## 审查范围

全仓清单见 `data/test-dedup-20260926/repo-inventory.json`：扫描 5,829 个受版本管理的源码/配置文件，识别 1,388 个包含测试或支持测试的文件，覆盖 25 个 Rust 领域目录、前端单测、浏览器、Python/CLI、CI。完整重复文件扫描未发现可整份无损删除的副本；逐组追踪重复辅助逻辑、旧假设与调用入口。

扫描时文件分类为 Rust 1,175、浏览器 51、前端单测 111、脚本 51。重复片段命中包含多个 Worker 的独立边界和迁移结构，不能据此删除各模块的权限、合同或历史数据升级验证。当前前端文件数已随并行任务增加。

## 已落地的清理

| 区域 | 处理 |
| --- | --- |
| Rust LLM 流式测试 | 合并两个事件收集器，边生产边消费有界队列；移除手工 stage-0 基线、放大队列规避背压及重复哈希校验，保留 10,000 个碎片和 1 MiB 数据完整性并默认执行；移除只为旧基线使用的 sha2 开发依赖 |
| 八类 Sidecar 进程测试 | 请求体上限从共享资源合同读取，删除各测试私有的 8/16/64 MiB 副本；LLM 旧 8 MiB 断言修正为实际 24 MiB 合同，保留严格 413 断言 |
| 请求拒绝边界 | 区分客户端在请求前拒绝非法上下文和 Server 返回失败事件；不把客户端错误包装成不存在的服务器事件 |
| RAG 恢复测试 | 强制结束旧进程后给恢复阶段建立新连接池，继续校验持久化、重建和检索结果 |
| 13 个 CLI 流程脚本 | 共用登录、健康/合同校验、私有配置路径和密码 stdin；共用 10 组相同参数定义；补齐独立运行的报告目录与外部 Provider 日志参数；临时配置留在报告目录内 |
| 浏览器启动器 | 五个入口共用 Vite 启动、就绪和有界退出；旧端口上的其他服务不能冒充就绪；浏览器清理失败仍终止自己启动的服务，Native 启动失败同样回收资源 |
| Mock 与运行器 | 删除闲置导入、重复目录/日志函数、固定历史 PID 清单和吞掉超限错误体的回退；按实际进程组/会话验证所有权；关闭 HTTPError，权限和证据读取错误显性化；Agent 末尾实时状态不再被当作新用户回合，删除主 Agent 的重复回合裁剪 |
| 前端用例/夹具 | 修正受外部运行模式变量影响的单测；显式展开高级参数；演示页使用可滚动区域；补齐明确的功能包偏好与事实反馈夹具，未知请求仍失败；按规范 JSON 独立复算 v2 夹具摘要，保留旧结果被拒绝的断言 |
| Agent 时长预算 | 在入库前同时校验 UTC 与单调时钟可表示范围；终态用量读取不再计算未来截止时间，活跃子 Run 仍继承祖先限制；修复极大预算失败后连线程和用量也无法读取的问题 |
| CI | Python 改用 unittest 发现机制，纳入新增的运行器检查；接入 Node 助手和已有 LLM/UI 浏览器检查；Linux manager 测试集中执行，移除两个矩阵中的重复 Linux 项，保留 macOS/Windows |

上一轮非线性用例合并及旧恢复脚本退役见 `test-dedup.md`。RAG/Agent/画布/项目族导出、多视频模式、Native 生命周期、数据库事务与版本/权限负例均有独有覆盖，继续保留。

关键实现位于 `scripts/tests/run_cli_contract_e2e.py`、`frontend/e2e/process.mjs`、`crates/platform/a4n-test-support/src/process.rs`、`crates/platform/a4n-domain/src/agent.rs` 和 `crates/platform/a4n-db/src/agent/budget.rs`。CI 接入点为 `.github/workflows/module-checks.yml`。

## 验证账本

- Python：`python-unit-verified.log`，157 项通过，ResourceWarning 按错误检查。
- Node 助手：5 项通过，含占用端口、进程有界清理、进度终态和诊断脱敏。
- Linux Service Manager：`repo-rust-third/a4n_service_manager.log`，43 项含 ignored 进程/mTLS 检查通过。
- `repo-rust-services/`：LLM 单元 19、LLM 进程 11、导入 1、导出 1、图片 1、视频 3、提示词 1 项通过。Embedding 初次缺少 ONNX 根变量，RAG 初次暴露强杀后旧连接池问题；修正后 `rag-embedding-stable/` 的 Embedding 2、RAG 1 项通过。连同 Service Manager 共 83 项。
- `agent-budget-validation/`：预算领域 2 项、真实 PostgreSQL 1 项通过，带修复的 Server/CLI/Worker/迁移已构建；本轮 Rust 定向合计 86 项。CLI lifecycle 同时确认拒绝非法预算时不新增 Run、消息或 Provider 请求，也不改写既有用户历史。
- `frontend-frozen/`：生产构建、5 项助手、缓存统计、视频模式 20 项、视频播放 13 项及 UI 的全部三组通过；预设页的不可点击问题和模式单测的环境依赖已修复。
- `frontend-final/`：两个 LLM 浏览器入口通过，包含 1440/1024/390px 横向溢出为 0、表单/键盘、空结果/错误恢复；独立截图审查未发现明显重叠、裁切或滚动问题。
- `frontend-unit-verified.log`：当前工作区 113 个前端测试文件、428 项全部通过。两条 v2 固定摘要由 Python 标准库独立复算并同步，证据 `shooting-fixture-digests.json`；未删除固定预期值或绕过内容变化校验。
- `node-helpers-verified.log`：5 项浏览器助手复核通过。CI YAML 解析通过，共 21 个 job。
- CLI：13 组共 121 项通过，其中 `repo-cleanup-20260926-verified` 的 12 组为 117 项；Agent domains 修正审批记录归属后，在 `repo-cleanup-20260927-agent-domains` 的全新隔离环境复核 4 项通过。前一份原始总报告仍保留失败状态，未改写历史失败。
- 浏览器：`repo-cleanup-20260926-verified/browser/stages.json` 的灵感、蓝图、大纲、正文、分析、导演、剧本、片段和媒体九阶段全部通过；Agent 定向复核还实际重走了前四阶段。浏览器访问生产构建，模型使用本地 Mock；CLI 包含严格取消、无效模型输出、缓存协议、断点恢复、媒体发布及审批负例。
- 当前源码包含其他会话正在修改的拍摄剧本合同。全仓 rustfmt 仍在 `project/family_package/tests/core_validation/text_core.rs` 和 `shooting_script_timing_v2/tests.rs` 报格式差异，本轮 Rust 修改文件未报差异。API 生成合同尚未同步；首次命令漏套共享构建锁，不计正式验收，随后持锁重跑因另一会话占用锁 60 秒退出 75，也不计通过。未自动重生成或整理他人合同改动。

## 版本与收尾

机器可读汇总为 `data/test-dedup-20260926/validation-final.json`。汇总检查了两轮被测版本一致、13 组实际成功结果和九阶段浏览器结果；它是复核后的账本，不是一次性全绿回执。两轮应用源码 SHA256 为 `879fe53561d91699a90b2cbba9d623ee1e25fbb50a353b0c5501a9737ad48318`，前端为 `a51c3c81a1e5d32ff908d014b161bd25f48ec7e24cf599490ebf855a7e356023`。该固定应用含本轮 Agent 预算修复，早于另一会话的拍摄 v2 合同变更；测试流程取当前脚本，Mock 的拍摄结构与被测应用固定匹配，没有给产品添加旧合同回退。

本轮 Rust 文件格式、Python 语法及 `git diff --check` 通过。本轮 Server、Worker、Mock、Qdrant、浏览器和 PostgreSQL 容器均已停止，证据 `resources-verified.json`。早期预算验证脚本曾遗漏 Docker 的 `-v` 参数，现已修正；因原容器已删除且 Docker 事件不再保留，无法证明可能遗留匿名卷的归属，未执行全局清理。共享构建锁的当前持有者和其他会话服务均未终止。

直接保留 `debug` 上本轮未提交改动，未新开分支，未整体提交、合并或推送；共享 WIP 和协调记录保留。当前整个 `debug` 尚不能宣称全部门禁通过。真实模型文学/导演质量、非线性 5 个指定模型场景、真正 Native IPC 及 macOS/Windows 未由本轮 Web/Mock 验证证明。所有原始失败、未执行和通过记录分别保留。
