<p align="center">
  <img src="frontend/src/assets/afn-logo.svg" alt="A4N" width="72" height="72">
</p>

<h1 align="center">A4N</h1>

<p align="center"><strong>从小说创作到漫画、视频与互动故事</strong></p>

<p align="center">中文 · <a href="README.en.md">English</a></p>

<p align="center">
  <a href="docs/public/getting-started.zh.md">快速开始</a> ·
  <a href="docs/public/assets/demo.mp4">观看 / 下载演示</a> ·
  <a href="#一眼了解-a4n">功能总览</a> ·
  <a href="#核心创作链路">创作链路</a> ·
  <a href="#核心设计">核心设计</a> ·
  <a href="https://github.com/GTJasonMK/A4N/issues">反馈问题</a>
</p>

<p align="center"><a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-34699A" alt="Apache License 2.0"></a></p>

A4N 把小说写作、故事资料管理和多媒体改编放在同一个创作体系里。你可以从灵感开始，也可以导入已有作品；逐步建立蓝图、大纲、正文和档案，再制作漫画、视频或互动故事。

> 项目仍在开发，目前需要从源码运行，尚无正式安装包。首次启动需要开发环境和 PostgreSQL，模型或 API 由你自行配置。

## 一眼了解 A4N

```mermaid
flowchart LR
  accTitle: A4N 功能总览
  accDescr: 四组能力分别覆盖小说写作、多媒体改编、自由画布和共享的创作支持。
  A4N["A4N"] --> writing["小说创作"]
  writing --> knowledge["灵感 · 蓝图 · 大纲<br/>正文 · 分析 · 档案<br/>事实反馈 · 前文检索"]
  A4N --> adapted["多媒体改编"]
  adapted --> works["漫画 · 视频 · 互动故事<br/>快照 · 素材引用 · 同步"]
  A4N --> canvas["自由画布"]
  canvas --> nodes["文字 · 图片 · 音视频<br/>节点编排 · 生成 · 处理"]
  A4N --> platform["创作支持"]
  platform --> tools["Agent · 协作 · 导入导出<br/>可恢复任务 · 多端入口"]
```

| 想了解什么 | 从这里看 |
| --- | --- |
| 从灵感写成小说，持续整理前文 | [小说写作](#小说写作) · [分析与事实反馈](#分析与事实反馈) |
| 准备图片、立绘与制作素材 | [图片与立绘](#图片与立绘) |
| 把故事制作成其他作品 | [漫画](#漫画) · [章节视频](#章节视频) · [互动故事](#互动故事) |
| 自己编排生成与素材处理步骤 | [自由画布](#自由画布) · [Agent 与自动化](#agent-与自动化) |
| 管理来源变化、团队和作品交付 | [来源同步](#来源同步) · [协作与导入导出](#协作与导入导出) |
| 了解系统如何运行和恢复任务 | [核心设计](#核心设计) · [快速开始](#快速开始) |

<details>
<summary>查看写作台截图与 50 秒操作演示</summary>

[![A4N 写作台：示例章节与内容审阅并排呈现](docs/public/assets/writing-desk.webp)](docs/public/assets/demo.mp4)

*《山海之间》演示项目。点击截图观看 50 秒操作演示。演示使用虚构故事、预置素材和模拟模型响应，详见[素材说明](docs/public/demo.md)。*

</details>

## 核心创作链路

下面的图按创作阶段展开。可以手工编辑、按需生成和查看已有结果，无需把整条链路一次跑完。

### 小说写作

```mermaid
flowchart TB
  accTitle: 从灵感到章节的小说创作流程
  accDescr: 灵感生成蓝图和大纲，也可导入已有正文；章节可分析后续写，或以作者确认的补丁优化。
  idea["灵感对话 / 故事构思"] --> blueprint["故事蓝图<br/>梗概 · 世界观<br/>人物 · 关系"]
  blueprint --> outline["分部大纲 → 正式章纲"]
  outline --> chapter["写作台<br/>手工写作 / 生成章节"]
  imported["录入 / 导入已有正文"] --> chapter
  chapter --> analysis["章节分析与资料反馈"]
  analysis --> next["结合前文资料继续写作"]
  next --> chapter
  chapter --> proposal["局部优化建议 → 差异预览"]
  proposal --> approval{"作者采纳且原文版本匹配？"}
  approval -->|是| chapter
  approval -->|否| retained["保留原文 / 重新提出建议"]
```

蓝图、大纲、章节和档案都可以查看与修改。局部优化先展示改前与改后内容，确认后才写回；旧提案不能覆盖已经变化的原文。

### 分析与事实反馈

```mermaid
flowchart TB
  accTitle: 章节分析、资料沉淀与后续写作的反馈循环
  accDescr: 分析结果回指原文，每章同步当前事实，按周期归纳资料；检索前文章节为后续生成准备上下文，未来剧情建议需作者采用。
  chapter["已保存正文 + 作者发起分析"] --> extract["摘要 · 事实 · 事件 · 伏笔<br/>关键结果附原文证据"]
  extract --> profiles["每章同步<br/>人物 · 物体 · 场景 · 伏笔"]
  extract --> index["分析记录与检索索引"]
  profiles --> consolidate["按周期 / 手动归纳<br/>世界事实 · 说话特征<br/>已发生的成长"]
  consolidate --> suggestions["梗概 / 未来剧情建议<br/>作者决定是否采用"]
  profiles --> context["当前章纲 + 写作要求<br/>前文资料"]
  index --> retrieval["按任务检索前文<br/>第 1 至 n−1 章<br/>匹配 Embedding 身份"]
  retrieval --> context
  consolidate --> context
  context --> generation["生成第 n 章<br/>优先连续性<br/>再补摘要与原文片段"]
  generation -.->|保存后可再次分析| chapter
```

低频资料默认累计 5 个不同章节后归纳，可调整周期；重要世界规则变化等情况会提前触发。已有正式章纲保留，缺少时补充回顾章纲。修改旧章会撤回失效来源，保留后续手工修改；普通保存不会触发模型调用。详见[正文事实反馈](docs/public/fact-feedback.zh.md)与[反馈示例图](docs/public/assets/self-feedback.zh.svg)。这里的反馈用于管理上下文，不会重新训练模型。

### 图片与立绘

```mermaid
flowchart TB
  accTitle: 图片、角色立绘与素材引用
  accDescr: 人物资料和创作要求用于生成或编辑提示词，图片模型生成的素材与上传图片进入项目素材库，再按工作台选择绑定并查看依赖。
  profiles["人物档案 + 创作要求"] --> prompt["生成 / 编辑立绘提示词"]
  prompt --> generation["图片模型生成<br/>角色立绘 / 创作图片"]
  generation --> library["项目图片与立绘库"]
  uploaded["上传已有图片"] --> library
  library --> reuse["按工作台选用或绑定<br/>漫画参考图 · 导演图像点<br/>互动素材"]
  reuse --> dependencies["查看素材依赖<br/>追踪引用与受影响产物"]
```

图片和立绘可以生成或上传，保留项目归属与引用关系。角色立绘支持重生成和年龄变体；各工作台按自身规则选择素材，系统参考图与用户添加图片按保存的绑定信息区分。

### 漫画

```mermaid
flowchart TB
  accTitle: 漫画从故事来源到整页成图的制作流程
  accDescr: 来源与素材经过预处理和改编，再进行分页、单页内容、分镜、参考图、提示词及成图；中间产物可查看和编辑。
  source["故事来源 + 基础素材<br/>人物 · 场景 · 图片"] --> prepare["漫画预处理与章节改编"]
  prepare --> pages["分页与每页内容<br/>单页改编计划"]
  pages --> storyboard["单页分镜<br/>格子 · 动作 · 对白 · 旁白"]
  storyboard --> references["发现与绑定参考图<br/>系统立绘 + 用户添加图片"]
  references --> prompt["整页绘图提示词"]
  prompt --> image["整页成图 → 查看 / 重生成"]
```

每一步保留结构化产物和来源。可以查看、编辑或删除指定阶段，删除前预览下游影响；系统参考图与用户添加图片分别管理。

### 章节视频

```mermaid
flowchart TB
  accTitle: 章节视频的导演、拍摄、生成与成片链路
  accDescr: 导演方案由作者确认，拍摄剧本生成共享片段计划，帧图和视频引用同一计划，结果进入时间线合成与后期。
  source["章节来源 + 基础素材"] --> director["导演方案<br/>镜头 · 时长<br/>图像点 · 调度"]
  director --> confirm["作者确认方案<br/>生成拍摄剧本"]
  confirm --> segments["共享片段计划<br/>来源身份 · 执行配方<br/>参考素材"]
  segments --> frames["生成 / 绑定帧图"]
  segments --> prompts["视频提示词与片段配置"]
  frames --> clips["按片段生成视频"]
  prompts --> clips
  clips --> assembly["时间线编排与合成"]
  assembly --> post["后期处理 / 输出成片"]
```

导演方案支持手工及 Agent 编辑，正式确认由作者完成。帧图与视频共用片段计划；首帧串联模式要求前一段尾帧。上游修改后检查对应片段的依赖关系，避免把旧素材直接用于新计划。详见[导演编辑设计](docs/public/director-native-editing.md)。

### 互动故事

```mermaid
flowchart TB
  accTitle: 互动故事的脚本、分支、素材与发布流程
  accDescr: 故事改编后设计分支，生成对白和演出配置，绑定素材，再在内置运行器中体验或导出 WebGAL。
  source["故事来源与改编计划"] --> branches["分支设计<br/>选择 · 条件 · 跳转"]
  branches --> script["场景脚本与对白"]
  script --> staging["人物演出与视觉规划"]
  staging --> assets["素材计划与绑定<br/>背景 · 立绘 · 图片 · 音频"]
  assets --> play["内置故事运行器<br/>推进剧情 / 选择分支"]
  assets --> export["导出 WebGAL 项目"]
```

可以修改场景、台词、顺序和选项，规划或生成图像素材，并导入音频。运行器与导出使用已保存的故事及素材绑定。

### 自由画布

```mermaid
flowchart LR
  accTitle: 自由画布的节点工作流
  accDescr: 文本和媒体作为节点输入，经过模型生成或媒体处理，再预览与输出；实际流程由用户连接的节点决定。
  text["文本输入"] --> llm["LLM 文本节点"]
  llm --> image["图片生成"]
  image --> video["视频生成"]
  imported["导入图片 / 音视频"] --> process["裁剪 · 抽帧 · 媒体处理"]
  video --> process
  process --> output["预览 / 输出节点"]
  llm --> output
  image --> output
```

图中是可编排的示例，不要求所有节点都执行。自由画布是独立项目；当前画布生成节点用于自由视频项目，章节派生视频走上面的章节工作流。

<details>
<summary>查看自由画布示例</summary>

[![自由画布：导入视频、裁剪、抽帧与输出](docs/public/assets/multimodal-workspace.webp)](docs/public/assets/multimodal-workspace.webp)

示例使用预置视频，展示素材处理，不代表实际调用了视频生成模型。

</details>

## Agent 与自动化

```mermaid
flowchart TB
  accTitle: Agent 从创作目标到可审阅结果的执行流程
  accDescr: 小说优化、导演和画布领域会话读取绑定上下文，发现工具和技能，按权限及模式执行；任务与审批可观察，正式正文补丁和导演确认仍由作者决定。
  goal["作者目标<br/>小说优化 · 导演编辑<br/>画布编排"] --> session["领域会话<br/>绑定项目与资源"]
  session --> tools["读取上下文 · 发现工具<br/>加载 Skills"]
  tools --> mode{"权限与执行模式"}
  mode -->|建议模式| proposal["只读分析 / 修改建议"]
  mode -->|需审批| review["作者批准工具操作"]
  mode -->|自主模式| execute["执行已授权的合法操作"]
  review --> execute
  execute --> result["草稿编辑 / 生成任务<br/>执行回执"]
  execute -.-> delegation["按需委派子任务"]
  delegation -.-> result
  proposal --> monitor["查看会话、任务与结果<br/>处理待审批操作<br/>取消运行任务"]
  result --> monitor
```

Agent 可以调用项目工具并处理长任务，CLI 也能发现命令、提交任务和观察 JSON/JSONL 事件。正文补丁采纳与导演方案正式确认仍由作者执行；不同领域的可用工具与操作模式以工作台为准。

## 来源同步

```mermaid
flowchart TB
  accTitle: 原稿更新与派生作品的来源同步
  accDescr: 派生项目保存正文、分析和档案快照，原稿变化后先比较差异，遇到本地修改由作者选择处理方式，再校验依赖并更新受影响内容。
  novel["小说正文 · 分析 · 档案"] --> snapshot["创建派生项目时<br/>保存来源快照"]
  snapshot --> derived["漫画 · 章节视频<br/>互动故事"]
  novel --> changes["原稿后续变化"]
  changes --> diff["比较新旧来源<br/>章节增删与顺序 · 正文<br/>分析 · 档案"]
  snapshot --> diff
  diff --> decision{"派生内容也有本地修改？"}
  decision -->|有| choose["作者选择处理方式"]
  decision -->|无| apply["确认同步"]
  choose --> apply
  apply --> dependencies["校验素材与下游依赖<br/>更新来源<br/>标记受影响产物"]
  dependencies --> derived
```

来源快照让“制作时用的是哪一版故事”可追溯。素材依赖图帮助查看来源和产物关系；原稿变化不会静默覆盖派生作品的本地编辑。自由画布不因此获得小说来源快照。见[多媒体来源示例图](docs/public/assets/multimodal.zh.svg)。

## 协作与导入导出

```mermaid
flowchart TB
  accTitle: 项目协作与作品交付
  accDescr: 已有文本或项目包可进入创作项目，协作者接受邀请后按角色访问，项目可导出阅读格式或受支持的项目与媒体包。
  imported["录入 / 导入正文 · 导入项目包"] --> project["创作项目与项目族"]
  project --> invite["邀请成员 → 接受邀请"]
  invite --> roles["管理者 / 编辑者 / 查看者<br/>按权限访问项目"]
  project --> text["小说交付<br/>TXT · Markdown · EPUB · PDF"]
  project --> records["结构化资料<br/>大纲 / 人物 JSON"]
  project --> package["受支持的项目 / 项目族包<br/>保留对应资料<br/>来源与媒体"]
  package --> transfer["导入另一环境 / 继续创作"]
```

成员权限围绕项目及其关联作品管理。导出阅读文件与导出可继续编辑的项目包用途不同；项目包只包含相应功能支持的内容，完整备份仍需同时保存 PostgreSQL 数据和媒体文件。

## 核心设计

### 多端共用业务核心

```mermaid
flowchart TB
  accTitle: 多端入口、业务核心、数据和模型能力的架构
  accDescr: 桌面默认连接本地 Server，Web 和 CLI 也通过 Server 进入共享业务核心；Tauri Native 是另一适配入口。业务通过各自边界访问数据库、索引、文件和模型能力，此图不是 Rust 直接依赖图。
  desktop["桌面端 · Tauri 2"] -->|默认| server["本地 / 部署的 Server"]
  web["Web 浏览器"] --> server
  cli["a4n CLI / 自动化"] --> server
  desktop -.->|可选| native["Tauri Native 适配"]
  server --> host["共享 Host API 合同<br/>入口适配"]
  native --> host
  host --> business["业务编排与规则<br/>写作 · 改编 · Agent · 工作流"]
  business --> db["PostgreSQL<br/>正文 · 档案 · 版本 · 任务"]
  business --> rag["Embedding + Qdrant<br/>前文检索索引"]
  business --> files["数据目录<br/>图片 · 音视频 · 导出文件"]
  business --> runtime["运行适配与能力服务"]
  runtime --> models["文本 / 图片 / 视频 Provider<br/>本地服务或外部 API"]
  runtime --> media["媒体处理<br/>ffmpeg 与对应 sidecar"]
```

箭头表示调用或数据流，不是 Rust 直接依赖。接口合同、业务规则、数据访问和运行适配各有归属；可选能力包按需启用，文本、Embedding、图片和视频服务分别配置。更细的职责划分见[架构示例图](docs/public/assets/architecture.zh.svg)。

### 可恢复任务与版本保护

```mermaid
flowchart TB
  accTitle: 长任务的检查点恢复和来源版本保护
  accDescr: 支持恢复的流程按输入身份复用完成步骤，失败保持可见；最终写入前校验正式来源版本，来源变化时拒绝保存旧结果。
  request["提交任务<br/>记录输入与来源版本<br/>保存运行配置"] --> task["任务 ID + 进度 / 事件"]
  task --> checkpoint{"有可复用的完成步骤<br/>且输入身份匹配？"}
  checkpoint -->|有| reuse["复用已完成结果"]
  checkpoint -->|无| run["执行剩余步骤"]
  reuse --> run
  run --> saved["持久化阶段结果与检查点"]
  saved --> version{"正式来源版本仍匹配？"}
  version -->|是| commit["写入正式结果"]
  version -->|否| conflict["拒绝旧结果写入<br/>提示刷新 / 重新生成"]
  run -->|失败或中断| stopped["记录明确状态与错误"]
  stopped -.->|流程支持恢复时| checkpoint
```

恢复能力取决于具体工作流和当前状态。正文、章纲或来源变化时，检查点不能绕过版本校验；失败、取消和冲突都有明确结果。

各图对应的代码入口与行为边界见[设计依据](docs/public/design-evidence.md)。公开演示的实际验证范围见[验证说明](docs/public/demo.md)。技术栈为 Rust、Tauri 2、React/TypeScript、PostgreSQL/SeaORM 和 Qdrant。

## 快速开始

1. 按[源码运行指南](docs/public/getting-started.zh.md)准备环境，启动桌面端或 Web 端。
2. 在设置中配置所需模型与服务。
3. 创建小说项目，从灵感与蓝图开始，或录入 / 导入已有章节。
4. 保存正文并发起分析，核对档案与反馈，再继续写作或创建多媒体作品。

| 要使用的能力 | 需要准备 |
| --- | --- |
| 小说写作、分析、内容优化、Agent | 文本模型 / API |
| 前文检索反馈 | Embedding 配置与 Qdrant |
| 图片、立绘、漫画成图 | 图片模型 / API 或支持的本地服务 |
| 视频生成与媒体处理 | 视频模型 / API；处理与合成需要 ffmpeg 及对应 sidecar |
| 互动故事 | 故事脚本与相关素材；运行、导出条件以工作台为准 |

模型和服务在应用设置中配置，见[配置指南](docs/public/configuration.zh.md)。格式、分辨率、时长和费用取决于实际 Provider。

## 常见问题

**需要支付模型费用吗？** A4N 源码采用 Apache 2.0。第三方 API、托管服务及本地模型的硬件成本由使用者承担；模型和素材自身的许可需分别确认。

**可以完全离线使用吗？** 选择外部模型时，请求所需的正文、上下文或媒体会发送到对应服务。要离线使用，需要在本地运行相应模型和服务。

**作品保存在哪里？** 结构化数据保存在配置的 PostgreSQL，媒体与本地运行数据保存在配置的数据目录。备份时需同时考虑数据库和文件；[配置指南](docs/public/configuration.zh.md)说明路径与边界。

**能通过脚本或 Agent 使用吗？** 内置 `a4n` CLI 连接同一 Server，提供命令发现、任务观察和 JSON/JSONL 输出。使用方式见[CLI 指南](docs/public/configuration.zh.md#无前端-cli)。

## 开发与反馈

从[贡献指南](CONTRIBUTING.md)开始；构建、打包及正式发布的边界见[发布说明](GITHUB_RELEASE.md)。问题反馈请附版本或提交、运行方式和可复现步骤，移除密钥与私人内容。安全问题按[安全策略](SECURITY.md)私下报告。

[Apache License 2.0](LICENSE) · [英文版](README.en.md) · [开始第一次创作](docs/public/getting-started.zh.md)
