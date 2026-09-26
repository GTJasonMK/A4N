# 创作阶段端到端验收

2026-09-26，直接修改 `debug`，未新建分支、提交或合并其他会话的 WIP。

已完成统一入口及实际验收。第 24 次完整运行退出码为 0，九阶段全部通过，五组 CLI 共 69 项检查通过。报告：[report.json](../../data/creation-e2e/dev-20260926-twentyfourth/report.json)。

## 使用

对启动时的当前工作区创建快照并验收：

```bash
python3 scripts/tests/run_creation_e2e.py
python3 scripts/tests/run_creation_e2e.py --stage inspiration
python3 scripts/tests/run_creation_e2e.py --list
```

当前环境使用 PostgreSQL 镜像 `docker.m.daocloud.io/library/postgres:16`，可用 `--postgres-image` 指定。完整依赖、阶段边界和报告语义见 [CREATION_E2E.md](../../scripts/tests/CREATION_E2E.md)。

复验本次已保存的应用版本，重新创建数据库及运行环境：

```bash
python3 scripts/tests/run_creation_e2e.py \
  --application-source data/creation-e2e/dev-20260926-twentyfourth/source \
  --binaries-dir data/creation-e2e/dev-20260926-twentyfourth/bin \
  --postgres-image docker.m.daocloud.io/library/postgres:16
```

`--application-source` 固定被测应用，测试脚本使用当前工作区；二进制必须通过源码及文件哈希校验。默认入口始终验收新采集的当前工作区。

## 验收的两端

入口是用户的输入、点击或 CLI 操作。出口包含正确持久化、刷新重读、编辑后的版本保护，以及下游实际消费；图片要解码，视频要被采用且刷新后完整播放。

| 阶段 | 本次真实浏览器终点 |
| --- | --- |
| 灵感 | 选项、文本输入、完成提示三种交互；历史刷新重读，旧选项禁用 |
| 蓝图 | 生成、精修、确认进入项目；新蓝图及角色可读 |
| 大纲 | 分部和章纲生成、手工编辑、刷新保留 |
| 正文 | 基于章纲生成、精确保存；桌面及 390px 编辑后刷新不丢文 |
| 分析 | 五类资料同步完成；正文仍保留作者修改 |
| 导演 | 镜头/场景绑定和 6 秒时长保存；键盘拖动播放位置、确认后重读 |
| 剧本 | 从确认方案生成场景，编辑场景标题并刷新重读 |
| 片段 | 消费已编辑剧本，生成规划，保存作者约束并刷新重读 |
| 媒体 | 首帧提示词→图片→最终提示词→视频→采用→刷新后完整播放 6 秒 |

截图包括 [窄屏正文](../../data/creation-e2e/dev-20260926-twentyfourth/browser/chapter-narrow.png)、[导演编排](../../data/creation-e2e/dev-20260926-twentyfourth/browser/director.png)、[已采用视频播放结束](../../data/creation-e2e/dev-20260926-twentyfourth/browser/media.png)。

CLI 补充检查版本/引用、实际 Provider 输入、错误/取消、断点复用、来源失效、导出导入等 UI 单条成功路径无法完整证明的不变量：

| 测试组 | 通过检查数 |
| --- | ---: |
| authoring：长对话、蓝图、大纲、正文、分析 | 14 |
| checkpoints：非法输出和调用级断点恢复 | 19 |
| facts：正文事实撤回、重建及来源 | 8 |
| director：导演、剧本、片段、媒体及项目族往返 | 17 |
| shooting：异步任务、观察者断线、停止和来源同步 | 11 |

Provider 实际记录 627 次请求，覆盖文本、向量、图片生成/编辑、视频生成。浏览器通过真实 UI 登录及配置个人密钥，没有用 API 写入冒充页面操作。

## 修复与同步

| 问题 | 修改位置与结果 |
| --- | --- |
| 章纲局部编辑在 Host→Worker 跳转中把缺省字段变成 `null`，产生 422 | [outline.rs](../../crates/host/a4n-api-contract/src/outline.rs)：序列化省略未提供字段，保留显式 null 拒绝；DTO 往返单测和真实编辑通过 |
| 无效生成输入被分类为可自动重试的持久化失败 | [context/generation/mod.rs](../../crates/host/a4n-orchestrator/src/context/generation/mod.rs)：InvalidArgument/InvalidState 使用 `generation_request_invalid`、不可重试 |
| 导出筛选对可空导演方案 ID 的类型使用错误 | [export/orchestration.rs](../../crates/platform/a4n-db/src/project/family/export/orchestration.rs)：按可空合同筛选，保留合法无绑定记录 |
| 视频来源投影仍引用已删除字段 | [shooting_script_projection.rs](../../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/frame_chain/shooting_script_projection.rs)：未发布正式剧本的来源明确为未完成 |
| 生成的 TS 合同缺少现有 Rust 的 obsolete 状态 | 用正式生成器更新 [host.ts](../../frontend/src/api/generated/host.ts) |
| 长正文输入触发状态栏重复注销/注册和 React 更新深度告警 | [StatusBar.tsx](../../frontend/src/components/ui/StatusBar.tsx)：绘制前更新内容，仅卸载时注销；同一长文本输入从 19 条告警降为 0，桌面/窄屏通过 |
| 选回同名生成方式时，输入方式不匹配也被提前返回 | [useVideoTabActions.ts](../../frontend/src/pages/novel-detail/video/useVideoTabActions.ts)：复用已有输入合同校验；显式选择时配置正确输入，保留有效独立首帧和取消语义；4 项回归及实际媒体链通过 |

本轮七个产品/生成文件均与已验证快照逐字一致：[文件校验记录](../../data/stage-e2e-development-20260926/validated-change-fingerprints.json)。其他会话的在途改动未覆盖。

## 测试本身的可靠性

- 空报告、缺检查、只通过 CLI、未运行、超时、上游失败都不能标绿；晚阶段失败保留早阶段的真实结果。
- 模型非法输出、取消和无个人密钥失败不保存半轮灵感；无密钥失败的 SSE 重放必须含失败事件且正常关闭。
- 分析输入去掉来源标记后，必须逐字等于已保存正文，避免把标记切分误当成缺失内容。
- 漫画恢复不会把上次失败终态当成本次结果；恢复未启动仍会超时失败。
- 对话框定位统一排除退出动画中的旧窗口，避免嵌套窗口定位到旧进度窗。
- 严重 JS 错误及未恢复的 HTTP 错误使测试失败；生成协议的 409 仅在同任务、同 attempt 后续快照 200 且业务断言通过时记录为已恢复。本次片段、媒体各验证了 2 次快照恢复，保留 `*-recoveries.json`。
- 资源清理失败也令整体失败，继续清理其他资源。仅按本轮目录和进程环境标记清理自己的服务。

辅助验证：2 项 Rust 定向回归、27 项前端检查、31 项 Mock Provider、28 项 CLI runner、3 项验收门禁、1 组浏览器诊断反向检查通过；生产前端、API/生成合同检查通过。全仓 rustfmt 首次发现两处并行格式改动；在对应文件更新后重新检查通过，未替其他会话整理源码。最终 `git diff --check` 通过。

本轮 PostgreSQL/Worker/API/Provider/Qdrant/Chromium 已停止；另外确认并关闭了第 7、8 次遗留的两个专用 ChromeDriver。共享编译 target 在独占锁内清理；源码、日志、媒体及数据库证据保留。

## 保证范围

被测后端 SHA256：`d53c8244ec629ad92c77b8bcdb26baea19358f4c6d57bac84527d1997c849b92`。

被测前端 SHA256：`7a548fbfbe7fb7344bccb8ef554b3542b73df0aba5420d43bb1ebbbdeb87d87d`。

报告的 `matches_current_worktree` 为 false：共享 debug 上其他会话仍在修改代码，前端目录也新增了本轮定向测试。绿色结果属于保存的完整应用快照；后续修改应重新运行入口，不能继承旧版本结果。

通过证明已定义场景与不变量，不证明不存在所有未知 bug。Mock 不证明真实模型的文学/导演/画面质量、所有厂商差异、全部历史数据升级、所有并发组合或 Native/Tauri IPC；这些需要各自的样本及测试。
