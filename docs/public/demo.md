# 素材与验证说明 / Media and validation notes

[中文首页](../../README.md) · [English README](../../README.en.md) · [演示视频 / Demo](assets/demo.mp4)

## 这段演示展示什么

演示使用真实 A4N Web 界面与隔离的 PostgreSQL、Qdrant 和本地服务，应用基线为 `e2f3aa0dcdd7a0ab613e986050eae568a9850c56`。故事《山海之间》、人物林岚与周衡，以及《未寄出的信》《旧书店》两章均为虚构示例。

正文预先录入；文本与 Embedding 响应来自确定性的本地测试 Provider。应用实际完成资料落库、索引检索、补丁持久化和作者确认。它展示操作与数据流，模型生成质量、检索质量和请求速度不在评测范围内。录像为静音剪辑，等待与页面切换有所压缩，中文字幕下附英文。

| 时间 | 实际操作 |
| --- | --- |
| 00:00–00:07 | 展示项目、章节与已保存正文 |
| 00:07–00:18 | 发起章节分析，查看五类资料同步与结构化结果 |
| 00:18–00:27 | 在下一章检索怀表与七分钟偏差的相关资料 |
| 00:27–00:41 | 审阅持久化补丁，确认后将修改写回正文 |
| 00:41–00:50 | 查看自由视频画布、本地裁剪与抽帧产物，并播放视频输出 |

## 素材来源

| 文件 | 来源与用途 |
| --- | --- |
| `self-feedback.*.svg` | 原生 SVG 方法图，区分作者发起、自动资料流与人工修订；内容例子是机制示意 |
| `multimodal.*.svg` | 原生 SVG 来源与派生关系图；不同模态独立进入，不表示必须逐项执行 |
| `architecture.*.svg` | 原生 SVG 架构概览；详细边界以链接的源码合同为准 |
| `story-reference.svg` | 为演示原创绘制的旧车站插画，作为预置参考；不是图片模型输出 |
| `writing-desk.webp` | 实际应用截图，显示第二章正文、补丁预览和优化助手 |
| `multimodal-workspace.webp` | 实际自由视频画布截图，使用预置视频完成裁剪、抽帧与输出 |
| `demo.mp4` | 实际浏览器操作录屏；1920 × 1080、静音、双语字幕 |

视频画布中的输入片段由预置插画制作，画布实际裁剪到四秒并提取一秒处的帧。自由画布中的素材是手工导入的；小说的派生项目与来源快照是另一条工作流。所有截图来自应用本身，未绘制替代界面或修改页面 DOM 来伪造运行状态。

## 验证范围与限制

- 已核对 CLI/Server 合同、首次账号与本机模型配置；实际完成正文保存、章节分析、五类资料同步、检索、可审阅补丁和媒体处理。机器可读结果见 [validation.json](validation.json)。
- 本轮验证使用隔离的 Web 环境与测试 Provider。没有覆盖全新操作系统安装、真实付费模型、所有模态生成、Tauri 原生窗口或正式发布包。
- 章节派生视频应使用派生项目的章节生成流程；当前画布生成节点仅允许自由视频项目。本次隔离运行的自由视频生成又遇到 `chapter_id 必须大于 0` 的校验错误，未记为通过。录像中的媒体流程使用明确标注的预置片段。
- CLI 的本机运行配置与浏览器设置需分别保存。本次曾因维度不一致出现同步失败，统一配置后通过通用 Host 调用完成同步；不把该结果扩展为所有 CLI 子命令均已覆盖。
- README 通过 GitHub Markdown API 渲染后在本地浏览器检查；实际 GitHub 页面的外层导航和主题样式由 GitHub 决定。素材纳入版本控制不等于仓库已经公开或发布。

## English

The recording uses the real A4N Web application, isolated PostgreSQL/Qdrant services and the application baseline shown above. **Between Mountains and Sea**, its characters and two chapters are fictional. Text is prepared in advance; a deterministic local provider supplies simulated text and embedding responses. The real application persists analysis, retrieves indexed material, stores a reviewable patch and applies it only after author approval.

The silent, edited recording has Chinese and English captions. Waiting and page transitions are condensed. It demonstrates the workflow, not model quality, retrieval quality or generation speed. The table above maps its five scenes: story (0–7s), analysis (7–18s), retrieval (18–27s), review (27–41s), and media (41–50s).

`story-reference.svg` is original, prepared illustration. A clip made from that still is imported into a **freeform** canvas; the application trims it to four seconds and extracts a frame at one second. These manually imported assets are distinct from chapter-derived projects and source snapshots. Screenshots show the actual UI; no substitute interface or DOM edits fabricate application state.

Validation covers the isolated Web workflow, local configuration, stored chapters, analysis and material synchronization, retrieval, patch review, media processing and playback. [validation.json](validation.json) records the measured results. It does not cover a fresh OS installation, paid providers, every modality, native Tauri windows or release installers. Chapter-derived video uses its own chapter workflow. Canvas generation currently requires a freeform project; the attempted generation in this environment failed a `chapter_id must be greater than 0` check and is **not counted as passing**.

CLI runtime configuration is separate from browser-local settings. An initial vector-dimension mismatch was resolved for the generic Host calls exercised here; this does not validate every CLI subcommand. README previews use GitHub's Markdown API and local browser rendering. GitHub controls its surrounding page styles. Tracking the assets does not publish the repository or create a release.

## README 组织参考 / README references

调研日期：2026-09-23。借鉴信息组织，不复制项目素材，也不以参考项目为 A4N 的质量背书。

- 真实工作台与体验入口 / Real UI and entry points: [ComfyUI](https://github.com/Comfy-Org/ComfyUI), [InvokeAI](https://github.com/invoke-ai/InvokeAI).
- 安装、模型与帮助 / Setup, models and help: [Dify](https://github.com/langgenius/dify), [Open WebUI](https://github.com/open-webui/open-webui).
- 场景化组织与一致视觉 / Use cases and visual consistency: [AFFiNE](https://github.com/toeverything/AFFiNE), [AppFlowy](https://github.com/AppFlowy-IO/AppFlowy), [LobeHub](https://github.com/lobehub/lobehub).
- 快速开始推进到第一个结果 / A first visible result: [Nerfstudio](https://github.com/nerfstudio-project/nerfstudio).
- 方法与效果分别呈现 / Separate method and output: [SAM 2](https://github.com/facebookresearch/sam2), [Wan2.2](https://github.com/Wan-Video/Wan2.2).
- 首页要回答的问题 / What a README should answer: [GitHub README guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes).
