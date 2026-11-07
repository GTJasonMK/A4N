<p align="center">
  <img src="frontend/src/assets/afn-logo.svg" alt="A4N" width="72" height="72">
</p>

<h1 align="center">A4N</h1>

<p align="center"><strong>从小说创作到漫画、视频与互动故事</strong></p>

<p align="center">中文 · <a href="README.en.md">English</a></p>

<p align="center">
  <a href="docs/public/getting-started.zh.md">快速开始</a> ·
  <a href="docs/public/assets/demo.mp4">观看 / 下载演示</a> ·
  <a href="#实际页面流程">页面截图</a> ·
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
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: A4N 功能总览
  accDescr: 小说可通过来源快照派生漫画、章节视频和互动故事。自由画布是独立入口，另有 Agent、协作、导入导出及可恢复任务等支持能力；无连线的功能块不表示执行顺序。
  subgraph story[" "]
    direction LR
    writing["小说创作<br/>灵感 / 蓝图 / 大纲<br/>正文 / 分析 / 档案<br/>事实反馈 / 前文检索"]
    writing --> adapted["多媒体改编<br/>基于来源快照<br/>漫画 / 章节视频<br/>互动故事"]
  end
  subgraph support[" "]
    direction LR
    canvas["独立自由画布<br/>文字 / 图片 / 音视频<br/>节点生成 / 媒体处理"]
    canvas ~~~ tools["创作支持<br/>Agent / 协作<br/>导入导出 / 任务恢复<br/>多端入口"]
  end
  story ~~~ support
  style story fill:transparent,stroke:transparent
  style support fill:transparent,stroke:transparent
```

这是功能分组图：小说与派生作品有来源关联，自由画布可以独立使用。下面各节再展开具体流程。

| 想了解什么 | 从这里看 |
| --- | --- |
| 从灵感写成小说，持续整理前文 | [小说写作](#小说写作) · [分析与事实反馈](#分析与事实反馈) |
| 准备图片、立绘与制作素材 | [图片与立绘](#图片与立绘) |
| 把故事制作成其他作品 | [漫画](#漫画) · [章节视频](#章节视频) · [互动故事](#互动故事) |
| 自己编排生成与素材处理步骤 | [自由画布](#自由画布) · [Agent 与自动化](#agent-与自动化) |
| 管理来源变化、团队和作品交付 | [来源同步](#来源同步) · [协作与导入导出](#协作与导入导出) |
| 了解系统如何运行和恢复任务 | [核心设计](#核心设计) · [快速开始](#快速开始) |

## 实际页面流程

用同一个虚构项目《逆行时钟》查看主要创作页面：先写小说、整理资料，再按需派生漫画、章节视频或互动故事；自由画布是独立入口。下面各功能配有关键页面，也可以按顺序阅读 [14 张截图的完整流程导览](docs/public/workflow-tour.md)。所有截图均可点击查看原图。

[![项目列表：逆行时钟的小说、漫画、章节视频、互动故事与独立自由画布](docs/public/assets/workflow/00-projects.webp)](docs/public/assets/workflow/00-projects.webp)

*项目管理：同一故事的小说与派生作品分别进入各自工作台，独立画布用于自由编排。*

截图来自真实应用与隔离演示数据，模型响应来自本地确定性测试 Provider，媒体使用预置示意素材，展示范围见[素材与验证说明](docs/public/demo.md#新增页面截图--workflow-screenshots)。

<details>
<summary>查看早期《山海之间》50 秒操作演示</summary>

[![A4N 写作台：示例章节与内容审阅并排呈现](docs/public/assets/writing-desk.webp)](docs/public/assets/demo.mp4)

*《山海之间》演示项目。点击截图观看 50 秒操作演示。演示使用虚构故事、预置素材和模拟模型响应，详见[素材说明](docs/public/demo.md)。*

</details>

## 核心创作链路

下面的流程图沿箭头阅读；长链路按阶段归组，框内箭头保留先后顺序。实线表示流程或数据流，虚线表示可选路径。可以手工编辑、按需生成和查看已有结果，无需把整条链路一次跑完。

### 小说写作

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 从灵感到章节的小说创作流程
  accDescr: 灵感可生成蓝图和大纲，再进入写作台；已有正文也可直接录入或导入写作台，不必经过蓝图生成。
  idea["灵感对话 / 故事构思"] --> plan["蓝图 → 分部大纲 → 正式章纲<br/>梗概 · 世界观 · 人物关系"]
  plan --> chapter["写作台<br/>手工写作 / 生成章节"]
  imported["录入 / 导入正文"] --> chapter
  chapter --> saved["保存章节<br/>按需分析、优化或继续写作"]
```

[![小说写作台：故事蓝图、角色资料、章节列表与已保存的雨夜来信正文](docs/public/assets/workflow/04-writing.webp)](docs/public/assets/workflow/04-writing.webp)

*写作台把蓝图、人物资料、章纲和正文放在一起；[完整导览](docs/public/workflow-tour.md)还展示灵感对话、蓝图和正式章纲。*

蓝图、大纲、章节和档案都可以查看与修改。局部优化单独走审阅流程：

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: 正文局部优化与作者采纳
  accDescr: 基于选段和原文版本提出补丁，作者先看差异；只有作者采纳且原文版本仍匹配时才写回。拒绝或版本过期保留原文。
  proposal["选定正文片段并记录版本<br/>提出补丁 → 预览改前 / 改后"]
  proposal --> approval{"作者采纳<br/>且版本匹配？"}
  approval -->|通过| applied["写回所审补丁"]
  approval -->|拒绝或过期| retained["保留原文"]
```

### 分析与事实反馈

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 章节分析与资料沉淀
  accDescr: 作者对已保存正文发起分析，提取结果保留原文证据，再同步档案、事实与检索索引。低频资料按周期或手动归纳，可能产生待作者采用的未来剧情建议。
  chapter["已保存正文<br/>作者发起分析"] --> material["提取摘要 · 事实 · 事件 · 伏笔<br/>关键结果附原文证据<br/>同步档案 / 事实 / 检索索引<br/>人物 · 物体 · 场景 · 伏笔"]
  material -.-> consolidate["按周期 / 手动归纳<br/>世界事实 · 说话特征<br/>已发生的成长"]
  consolidate -.-> suggestions["梗概 / 未来剧情建议<br/>等待作者决定是否采用"]
```

沉淀的资料在后续写作中按当前任务取用：

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 前文资料如何进入后续章节生成
  accDescr: 当前章纲和写作要求驱动前文检索，范围限定为第一至 n−1 章且 Embedding 身份匹配。检索结果与档案事实组成上下文，再生成第 n 章；保存后由作者按需发起下一次分析。
  intent["第 n 章章纲 + 写作要求"] --> retrieval["按任务检索前文<br/>第 1 至 n−1 章<br/>匹配 Embedding 身份"]
  retrieval --> context["结合档案与事实组装上下文<br/>连续性 → 摘要 → 原文片段"]
  context --> saved["生成并保存第 n 章<br/>作者可继续发起分析"]
```

低频资料默认累计 5 个不同章节后归纳，可调整周期；重要世界规则变化等情况会提前触发。已有正式章纲保留，缺少时补充回顾章纲。修改旧章会撤回失效来源，保留后续手工修改；普通保存不会触发模型调用。详见[正文事实反馈](docs/public/fact-feedback.zh.md)与[反馈示例图](docs/public/assets/self-feedback.zh.svg)。这里的反馈用于管理上下文，不会重新训练模型。

[![章节分析：正文索引、章纲与伏笔、物品、角色和场景五类资料同步完成](docs/public/assets/workflow/05-analysis.webp)](docs/public/assets/workflow/05-analysis.webp)

*分析结果与资料同步状态直接显示在章节旁，方便核对摘要、事件、伏笔及人物变化。*

### 图片与立绘

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 图片、角色立绘与素材引用
  accDescr: 通用图片使用自定义提示词，角色立绘使用人物档案与立绘提示词，各自从对应入口生成。已有图片或立绘可直接上传，无需模型生成；保存后按工作台规则选用和绑定。
  generation["通用图片：自定义提示词<br/>角色立绘：档案 + 立绘提示词<br/>按入口生成 · 立绘支持年龄变体"] --> library["保存图片或立绘记录<br/>保留项目归属"]
  uploaded["上传图片<br/>或角色立绘"] --> library
  library --> reuse["按工作台规则选择与绑定<br/>漫画 · 导演 · 互动素材"]
  reuse -.-> dependencies["按需查看素材依赖"]
```

图片和立绘可以生成或上传，保留项目归属与引用关系。角色立绘支持重生成和年龄变体；各工作台按自身规则选择素材，系统参考图与用户添加图片按保存的绑定信息区分。

### 漫画

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 漫画从故事来源到整页成图的制作流程
  accDescr: 来源与素材先经过漫画预处理和章节改编计划采纳，再分页并准备每页内容和单页改编，之后制作分镜、发现参考图、编写提示词和生成整页图片。
  source["1 前期准备<br/>来源 / 素材 → 漫画预处理<br/>制定并采纳章节改编计划"]
  source --> pages["2 页面设计<br/>分页 → 每页内容 / 单页改编<br/>分镜：格子 · 动作<br/>对白 · 旁白"]
  pages --> references["3 参考素材<br/>发现需求 → 绑定已有图片<br/>系统参考 / 用户补充"]
  references --> image["4 整页成图<br/>绘图提示词 → 生成图片"]
```

每一步保留结构化产物和来源。系统参考图与用户添加图片分别管理，参考发现阶段只识别需求和匹配已有素材。已有阶段产物需先删除才能重新生成，删除前预览下游影响。

[![漫画分镜规划：两页的阶段进度、单页分镜及系统与用户参考图入口](docs/public/assets/workflow/07-manga.webp)](docs/public/assets/workflow/07-manga.webp)

*这里展示已保存的分镜与参考需求检查；当前成图为 0/2，尚未生成漫画图片。*

### 章节视频

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 章节视频的导演、拍摄、生成与成片链路
  accDescr: 作者确认导演方案并准备拍摄剧本，共享片段计划确定提示词和执行配方。需要图像引导时准备对应帧图或参考素材；纯文本方案不依赖帧图，两条路径均校验输入后生成视频，再编排和输出。
  plan["1 导演与剧本<br/>章节来源 / 素材 → 导演方案<br/>作者确认 → 生成拍摄剧本"]
  plan --> segments["2 片段规划<br/>共享计划 · 提示词 · 执行配方<br/>保留来源身份"]
  segments --> inputs{"图像引导？"}
  inputs -->|需要| frames["准备帧图 / 参考素材"]
  inputs -->|纯文本| assembly["3 制作与交付<br/>校验输入 → 分段生成视频<br/>时间线编排合成 → 后期 / 输出"]
  frames --> assembly
```

导演方案支持手工及 Agent 编辑，正式确认由作者完成。帧图与视频共用片段计划；首帧串联模式要求前一段尾帧。上游修改后检查对应片段的依赖关系，避免把旧素材直接用于新计划。详见[导演编辑设计](docs/public/director-native-editing.md)。

[![章节视频导演工作台：已确认的六秒镜头、三个图片点、时间线和已绑定预置示意图](docs/public/assets/workflow/08-director.webp)](docs/public/assets/workflow/08-director.webp)

*导演时间线将镜头、时长与图片点放在同一视图；图中绑定的是预置示意图。[完整导览](docs/public/workflow-tour.md)接着展示确认后的拍摄剧本，本轮未生成视频成片。*

### 互动故事

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: 互动故事的脚本、分支、素材与发布流程
  accDescr: 故事改编后设计分支，生成对白和演出配置，绑定素材，再在内置运行器中体验或导出 WebGAL。
  branches["故事来源与改编计划<br/>设计选择 / 跳转分支"] --> script["场景脚本与对白<br/>人物演出与视觉规划"]
  script --> assets["素材计划与绑定<br/>背景 · 立绘 · 图片 · 音频"]
  assets --> play["内置运行器<br/>推进剧情 / 选择"]
  assets --> export["导出 WebGAL"]
```

可以修改场景、台词、顺序和选项，规划或生成图像素材，并导入音频。运行器与导出使用已保存的故事及素材绑定。

[![互动故事编辑器：五个节点、五条连线、一个选择分支及台词编辑](docs/public/assets/workflow/10-interactive.webp)](docs/public/assets/workflow/10-interactive.webp)

*同一章故事改编为可选择的叙事路径；左侧查看分支，右侧编辑节点对白和选项。*

### 自由画布

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart LR
  accTitle: 自由画布的文本与图片生成示例
  accDescr: 文本输入经 LLM 整理为提示词，再进入图片生成并连接图片输出。这是满足节点端口类型的一个示例，视频生成另需文本提示词及所选模式要求的图像输入。
  text["文本<br/>输入"] --> llm["LLM<br/>提示词"]
  llm --> image["图片<br/>生成节点"]
  image --> output["图片<br/>输出节点"]
```

媒体处理按输入类型连接，下面是视频裁剪与抽帧示例：

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart LR
  accTitle: 自由画布的视频处理示例
  accDescr: 导入视频后裁剪，可分别连接视频输出或抽帧节点；抽帧产生图片，连接图片输出，音频和图片不能直接进入视频裁剪节点。
  imported["导入视频"] --> trim["视频裁剪"]
  trim --> video["视频输出"]
  trim --> frame["抽帧"]
  frame --> image["图片输出"]
```

图中是两条可编排的示例，输出节点按文字、图片、音频和视频分别处理。视频生成必须连接文本提示词；仅提示词、首帧、首尾帧或参考图模式各有对应输入要求，图片不能单独代替提示词。自由画布是独立项目；当前画布生成节点用于自由视频项目，章节派生视频走上面的章节工作流。

[![自由画布：预置视频经裁剪后连接视频输出与抽帧，四个执行节点均已完成](docs/public/assets/workflow/12-canvas.webp)](docs/public/assets/workflow/12-canvas.webp)

*独立画布实际完成预置视频的裁剪、抽帧与输出，右侧显示 4/4 已完成；输入节点保持“就绪”。这里展示媒体处理，未调用视频生成模型。*

## Agent 与自动化

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: Agent 从创作目标到可审阅结果的执行流程
  accDescr: 领域会话绑定项目与资源，读取上下文并发现工具与技能。每次调用按权限、执行模式和工具审批策略检查，拒绝或不允许时不会执行该调用，通过后编辑草稿、生成任务或按需委派并验收子任务。
  goal["目标与领域会话<br/>小说优化 / 导演 / 画布<br/>绑定项目与资源"]
  goal --> policy["读取上下文 · 发现工具 / Skills<br/>检查权限、模式和工具策略<br/>需要审批时由作者决定"]
  policy --> allowed{"获准？"}
  allowed -->|否| blocked["拒绝该调用<br/>记录原因"]
  allowed -->|是| result["执行工具：编辑草稿 / 提交任务<br/>执行回执 / 可审阅结果<br/>按需委派并验收子任务"]
```

| 执行模式 | 对写操作的约束 |
| --- | --- |
| 建议 `advice_only` | 禁止写工具，只能读取、分析和给出建议 |
| 写入审阅 `review_writes` | 通常先审批；仅保存待审补丁的 `edit_article` 不直接改正文 |
| 自主 `autonomous` | 仍检查权限及工具策略，部分操作仍需作者审批 |

Agent 会话可查看任务、处理待审批操作并取消运行任务，CLI 也能发现命令、提交任务和观察 JSON/JSONL 事件。正文补丁采纳与导演方案正式确认仍由作者执行；不同领域的可用工具与操作模式以工作台为准。

## 来源同步

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 原稿更新与派生作品的来源同步
  accDescr: 派生时保留来源快照，原稿变化后预览差异与依赖影响，由作者逐项决定保留、合并、替换或其他受支持策略。应用时校验目标并更新快照关联与同步状态，未解决冲突继续显示，不自动重做所有媒体产物。
  snapshot["派生作品保存来源快照<br/>正文 · 分析 · 档案"] --> diff["原稿变化 → 比较快照与来源<br/>预览差异、依赖和本地修改"]
  diff --> choose["作者逐项选择处理策略<br/>保留 / 合并 / 替换等"]
  choose --> status["校验目标 → 应用同步计划<br/>更新快照关联与同步状态<br/>继续显示未解决的冲突"]
```

来源快照让“制作时用的是哪一版故事”可追溯。素材依赖图帮助查看来源和产物关系；原稿变化不会静默覆盖派生作品的本地编辑。同步不等于重生成全部媒体，支持的重生成策略由作者显式选择；解除来源关联也有独立策略。自由画布不因此获得小说来源快照。

[![来源更新预览：漫画项目检测到原稿变化，显示待处理差异与保留当前、应用安全更新操作](docs/public/assets/workflow/14-source-sync.webp)](docs/public/assets/workflow/14-source-sync.webp)

*在小说正文补充一句后，派生漫画检测到一条来源变化；此图停留在比较与选择阶段，尚未应用同步。*

## 协作与导入导出

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart LR
  accTitle: 项目协作与角色权限
  accDescr: 所有者或有权限的管理者邀请成员，成员接受后以管理者、编辑者或查看者角色访问。每次操作检查相应权限，所有者身份不通过成员邀请授予。
  owner["所有者<br/>或授权管理者"] --> invite["邀请 → 接受<br/>管理 / 编辑<br/>查看"]
  invite --> access["项目 / 项目族<br/>逐次校验权限"]
```

作品交付与可编辑项目迁移是两种用途：

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: 阅读文件交付与项目包迁移
  accDescr: 当前项目可导出阅读文件或结构化资料，也可导出受支持的可编辑项目包。只有项目包路径表示在另一环境导入并继续创作，阅读格式不等于完整项目备份。
  project["当前创作项目"] --> purpose{"交付用途？"}
  purpose -->|阅读 / 资料| documents["TXT / MD / EPUB / PDF<br/>大纲 / 人物 JSON"]
  purpose -->|继续编辑| transfer["受支持的项目 / 项目族包<br/>导入另一环境继续创作"]
```

成员权限围绕项目及其关联作品管理。导出阅读文件与导出可继续编辑的项目包用途不同；项目包只包含相应功能支持的内容，完整备份仍需同时保存 PostgreSQL 数据和媒体文件。

[![小说导出：选择 TXT 纯文本并显示下载完成与文件名](docs/public/assets/workflow/13-export.webp)](docs/public/assets/workflow/13-export.webp)

*从作品交付到本地文件：此处实际下载了 TXT，并核对文件包含最新保存的完整章节正文。*

## 核心设计

### 多端共用业务核心

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: 多端入口如何进入共享业务核心
  accDescr: 桌面默认模式、Web 和 CLI 通过 Server 服务组进入业务核心；桌面可选 Native 模式走 Tauri 命令适配。两条入口受共享 Host API 合同约束，合同本身不是独立运行服务。
  clients["桌面默认 / Web / CLI"] --> server["Server 服务组<br/>HTTP 命令 / 事件"]
  server --> business["共享业务核心<br/>写作 · 改编<br/>Agent · 工作流"]
  native["桌面 Native 模式<br/>可选入口"] -.->|Tauri| business
```

两条入口受共享 Host API 合同约束；合同是接口定义，不是中间运行服务。Server 内部由 Edge、Control 和 Runtime / Worker 协作，下面只画业务职责与资源关系：

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: 共享业务核心的数据与能力边界
  accDescr: 业务编排通过数据访问层管理 PostgreSQL、Qdrant 检索索引和媒体文件，通过运行适配调用模型 Provider 与媒体处理。这是职责和资源关系图，不是进程部署图或 Rust 依赖图。
  business["业务编排与规则"] --> data["数据与索引<br/>PostgreSQL<br/>Qdrant · 媒体文件"]
  business --> runtime["运行适配<br/>文本 · 图片 · 视频<br/>ffmpeg · sidecar"]
```

箭头表示调用或数据流，不是 Rust 直接依赖。接口合同、业务规则、数据访问和运行适配各有归属；可选能力包按需启用，文本、Embedding、图片和视频服务分别配置。模型可使用本地服务或外部 API。

### 可恢复任务与版本保护

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8, wrappingWidth: 240 }
---
flowchart TB
  accTitle: 长任务的检查点恢复和来源版本保护
  accDescr: 对支持恢复的工作流逐阶段检查输入身份；匹配的完成结果可复用，否则执行当前阶段。成功后经过该阶段写入校验，保存阶段结果并推进或完成；失败及取消保留明确状态。不是先完成所有阶段才统一写入。
  request["提交 / 恢复受支持的任务<br/>记录阶段输入、来源版本与配置"] --> checkpoint{"可复用？"}
  checkpoint -->|匹配| reuse["读取完成结果"]
  checkpoint -->|不匹配| run["执行当前阶段<br/>来源版本校验通过后<br/>保存阶段产物与检查点"]
  reuse --> next["推进下一阶段 / 完成任务"]
  run --> next
```

恢复能力取决于具体工作流和当前状态；所有阶段已有有效结果时可以直接收尾，无需强行执行“剩余步骤”。各阶段按自身写入规则保存产物，检查点不等于正式产物；正文、章纲或来源变化时不能绕过版本保护。

| 遇到的情况 | 处理结果 |
| --- | --- |
| 执行失败或任务取消 | 保留明确状态、错误或取消原因；已完成阶段按流程规则保留 |
| 正式来源版本变化 | 拒绝旧结果写入，提示刷新或重新生成 |
| 请求恢复任务 | 按当前来源与输入重新检查检查点，不保证任何状态都可恢复 |

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
