<p align="center">
  <img src="frontend/src/assets/afn-logo.svg" alt="A4N" width="72" height="72">
</p>

<h1 align="center">A4N</h1>

<p align="center"><strong>具备自反馈系统的本地优先小说与多模态创作工作台</strong></p>

<p align="center">中文 · <a href="README.en.md">English</a></p>

<p align="center">
  <a href="docs/public/getting-started.zh.md">快速开始</a> ·
  <a href="docs/public/assets/demo.mp4">观看 / 下载演示</a> ·
  <a href="#核心设计">核心设计</a> ·
  <a href="https://github.com/GTJasonMK/A4N/issues">反馈问题</a>
</p>

<p align="center"><a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-34699A" alt="Apache License 2.0"></a></p>

A4N 围绕一个故事组织创作：从灵感、蓝图和章节写作，到人物与场景档案，再到图片、漫画、视频和互动叙事。章节分析会沉淀为可检索的资料，参与后续写作；内容优化以可审阅的修改建议交给作者决定。

[![A4N 写作台：示例章节与内容审阅并排呈现](docs/public/assets/writing-desk.webp)](docs/public/assets/demo.mp4)

*真实应用界面，虚构演示项目《山海之间》。点击图片观看或下载短演示；[素材与验证说明](docs/public/demo.md)区分预置示例、模拟模型响应和真实应用操作。*

> **当前状态：活跃开发中，通过源码运行。** 首次启动需要开发环境和 PostgreSQL；AI 能力需要自行配置模型或 API。当前没有正式 Release 安装包。

## 为持续创作而设计

- **有依据的故事记忆。** 关键提取回指原文，分析后自动同步资料；下一章围绕章纲、人物与伏笔检索前序内容，优先组织连续性信息。
- **有来源的多模态派生。** 图片、漫画、视频与互动叙事复用故事资料；派生项目保留来源快照，改稿后可以检查影响，再决定如何更新。
- **可审阅、可继续的创作流程。** 优化建议先保存为补丁，作者确认后写回；支持恢复的工作流在输入匹配时复用已完成阶段。

## 核心设计

点击方法图可打开原尺寸 SVG，查看细节。

### 小说自反馈：让前文成为下一章的依据

[![自反馈方法图：用户发起章节分析，资料沉淀后检索进入后续写作；正文修订需要作者确认](docs/public/assets/self-feedback.zh.svg)](docs/public/assets/self-feedback.zh.svg)

你发起章节分析后，事实、摘要、人物状态与伏笔分阶段提取；关键结果通过原文片段引用落到具体出处。工作流继续同步回顾章纲、人物与物体档案、场景资料和检索索引，并保留作者已有的正式章纲。

生成下一章前，系统根据章纲、写作要求、相关人物、待回收伏笔等构造多路检索查询，范围限制在**当前章之前**。取回的资料按**关键连续性、相关摘要、参考原文**分配上下文预算。内容优化则在读到的原文范围内提出局部补丁，经作者确认并检查源内容后写回。

这是**创作资料与上下文的反馈**。模型不会因此自动训练，也不保证每次生成都能消除设定冲突；作者仍需审阅内容。图中的怀表线索是机制示意。

### 全模态创作：同一故事，多种作品

[![多模态来源图：小说与档案连接共享素材，分支派生图片、漫画、视频和互动叙事](docs/public/assets/multimodal.zh.svg)](docs/public/assets/multimodal.zh.svg)

图像与立绘留在项目中供后续选用；漫画、视频和互动叙事项目基于选定章节的**来源快照**独立制作。源正文、分析或档案变化后，可以比较新旧版本并检查受影响对象；已有本地修改的产物保留作者决策环节。

视频制作的帧图与片段共用**片段规划和执行身份**，参考素材需要匹配相应规划；首帧链通过前段尾帧衔接下一段。各阶段的中间产物可以按工作流继续处理，实际支持取决于所选模式和模型能力。

[![A4N 自由视频画布：为同一示例故事编排预置视频、裁剪、抽帧与输出](docs/public/assets/multimodal-workspace.webp)](docs/public/assets/multimodal-workspace.webp)

*自由视频画布中的预置示意素材。章节派生视频在派生项目的章节工作流中生成；当前画布生成节点仅支持自由视频项目。*

| 想做什么 | 需要准备什么 |
| --- | --- |
| 小说写作、分析、内容优化 | 文本模型；检索反馈另需 Embedding 配置与 Qdrant |
| 图片、立绘、漫画素材 | 已配置的图片模型/API，或对应的本地生成服务 |
| 视频生成与成片 | 视频模型/API；本地媒体处理需要 ffmpeg 及对应 sidecar |
| 互动叙事 | 故事脚本与相关素材；导出目标及条件以对应工作台为准 |

具体服务与参数在应用设置中配置，见[配置与本地数据](docs/public/configuration.zh.md)。不同模型的格式、分辨率、时长、价格及可用能力不同。

## 快速开始

1. 按[源码运行指南](docs/public/getting-started.zh.md)准备环境并启动桌面端或 Web 端。
2. 在设置中配置文本模型和 Embedding；图片、视频服务按需要添加。
3. 创建小说项目，准备蓝图和章纲，录入或导入第一章。
4. 发起章节分析，查看档案、伏笔与资料同步结果，再继续写作或审阅优化建议。

指南附有可直接录入的虚构示例及完成检查。完整安装命令、首次账号配置和排障都集中在那里。

## 为长流程创作提供工程支撑

[![架构图：桌面、浏览器和 CLI 通过 Host 进入共享业务核心，再连接数据与模型能力](docs/public/assets/architecture.zh.svg)](docs/public/assets/architecture.zh.svg)

桌面默认通过本地 Server 访问业务；Native 模式使用 Tauri 入口。浏览器和 CLI 也进入共享 Host API。命令合同、业务编排、数据一致性和能力适配各有明确归属。

长流程保存阶段结果与输入指纹。发生中断后，支持恢复的工作流会先核对输入，再复用已完成调用、继续剩余步骤；生成期间正式资料发生变化时，旧输入对应的结果会被拦截。

| 设计 | 对创作的意义 | 实现 / 验证入口 |
| --- | --- | --- |
| 共享 Host 合同 | 多种入口使用同一套业务命令定义 | [Host API 合同](crates/host/a4n-api-contract)、[一致性检查](frontend/package.json) |
| 可审阅补丁与冲突检查 | 已编辑的正文不会被过期补丁直接覆盖 | [补丁规则与测试](crates/host/a4n-orchestrator/src/optimize/tests/patch_flow.rs) |
| 工作流检查点 | 在满足恢复条件时复用已完成步骤 | [章节分析恢复规则](crates/host/a4n-orchestrator/src/chapter_analysis/workflow/checkpoint.rs) |
| 项目来源与快照 | 比较源稿变化，为已编辑产物保留处理选择 | [来源差异](crates/platform/a4n-db/src/source_sync/snapshots)、[决策保护](crates/platform/a4n-db/src/source_sync/decisions/guards.rs) |

更多实现入口与适用范围见[设计依据](docs/public/design-evidence.md)，本次实际运行范围见[素材与验证说明](docs/public/demo.md)。技术栈为 Rust、Tauri 2、React/TypeScript、PostgreSQL/SeaORM 与 Qdrant；模块职责及开发命令见[贡献指南](CONTRIBUTING.md)。

## 常见问题

**需要支付模型费用吗？** A4N 源码采用 Apache 2.0。第三方 API、托管服务及本地模型的硬件成本由使用者承担；模型和素材自身的许可需分别确认。

**本地优先等于完全离线吗？** 本地数据与运行方式由你配置。选择外部模型时，请求所需的正文、上下文或媒体仍会发送到对应服务；离线创作依赖已部署且支持所需能力的本地服务。

**作品保存在哪里？** 结构化数据保存在配置的 PostgreSQL，媒体与本地运行数据保存在配置的数据目录。备份时需同时考虑数据库和文件；[配置指南](docs/public/configuration.zh.md)说明路径与边界。

**能通过脚本或 Agent 使用吗？** 内置 `a4n` CLI 连接同一 Server，提供命令发现、任务观察和 JSON/JSONL 输出。使用方式见[CLI 指南](docs/public/configuration.zh.md#无前端-cli)。

## 开发与反馈

从[贡献指南](CONTRIBUTING.md)开始；构建、打包及正式发布的边界见[发布说明](GITHUB_RELEASE.md)。问题反馈请附版本或提交、运行方式和可复现步骤，移除密钥与私人内容。安全问题按[安全策略](SECURITY.md)私下报告。

[Apache License 2.0](LICENSE) · [英文版](README.en.md) · [开始第一次创作](docs/public/getting-started.zh.md)
