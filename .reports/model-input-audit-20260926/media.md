# 图片与视频最终请求

本轮区分“生成媒体提示词的 LLM”与“实际生成媒体的模型”。图片与视频 Provider 不需要再次接收整章、完整拍摄剧本、分析记录或缓存 hash。以下映射来自当前源码；真实网关是否支持扩展字段、最终成图/成片质量不由 Mock 验证替代。

## 已修复

| 问题 | 当前处理 | 关键文件 |
|---|---|---|
| CLIProxy/Seedance 丢失 negative | 在共享 `video_provider_prompt` 中把独立 negative 合入最终文本一次；应用层与 Provider 使用同一规则 | `crates/video/a4n-video-provider-contract/src/lib.rs`；两个 Provider 的 `request.rs` |
| CLIProxy 按不完整文本检查 4096 字节 | 对合入 negative 后的真实 UTF-8 文本计数；超限明确失败，不截尾部 | 同上；`video/provider/request_rules.rs` |
| 视频帧 negative 只存 metadata | batch item 接受并校验 `negative_prompt`，后端合成到图片 prompt | `media/image/batch/rules.rs`；`media/image/generation/application/generation.rs` |
| 单帧/分镜板规则只在前端追加 | 后端统一合成单图/多格规则；前端只传作者文字、negative 和引用 | `frontend/src/pages/novel-detail/video/videoFramePromptUtils.ts` |
| 导演图片点携带整章和编辑器对象 | 只投影当前静态点、当前镜头描述及绑定场景视觉信息，保留真实参考图 | `chapter_visual_plan/generation.rs` |
| 场景素材携带实体跟踪 ID/动作状态 | current_state 只取 summary、weather_or_lighting、mood；保留布局和可见环境约束 | `profile_material.rs` |
| 画像 LLM 重复人物身份字段 | 稳定包保留身份；变体动态区不再重复五项人物字段 | `portrait/prompt_generation/{payload.rs,request.rs}` |
| 单目标续接丢失来源文本 | 根据 execution recipe 的显式来源视频读取其不可变生成提示词；同章/完成态检查与 Provider 共用 | `video/chapter_video_generation/generation.rs`；`video/prompts/generation/frame_prompts/video_chain/` |

原样本中 `gv_9` / `gv_11` 各有 65 / 51 字的 negative，但实际请求未发送；这是可确认的字段丢失，不是对成片质量的推断。原主提示词分别已占 3,720 / 3,913 字节，故合入 negative 后仍必须执行最终上限检查。

## 视频后端合同

| 后端 | 文本/参考媒体 | negative | fps |
|---|---|---|---|
| openai-compatible-video | JSON prompt、首尾帧、reference_image_urls | 独立字段 | 发送 |
| newapi-video | JSON prompt、单首帧；尾帧/普通参考图显式拒绝 | 独立字段 | 当前不发送 |
| gemini-veo | instances prompt；首尾帧/最多三张参考图 inlineData | negativePrompt | 当前不发送 |
| seedance2-video | prompt、首尾帧、分组 input_reference、seconds/size | 合入 prompt | 当前不发送 |
| cliproxy-openai-video | prompt；按路由映射 image/input_reference/reference_images | 合入 prompt并计入字节上限 | 当前不发送 |
| openai-chat-video | user 文本含尺寸/时长/fps；图片为网关扩展顶层字段 | 顶层字段 | 文本及顶层发送 |

这些是仓库的适配合同，不是对任意同名服务商的能力保证。尤其 Chat 视频的图片依赖网关扩展，不能因它兼容文本 Chat 就认定支持这些图片字段。`fps` 未发送的后端不能承诺采用配置值。

首帧、首尾帧、分镜板、keyframes、纯文本和参考媒体模式继续遵守 execution recipe。实际图槽位说明与真正选择的媒体对应；分镜板保持“参考板”角色，不当作视频第 0 秒画面。输入不足、来源失效、引用不受支持和文本超限仍是显式失败。

## 图片调用最小输入

| 入口 | 当前保留 |
|---|---|
| 普通单图/批量图 | 用户文字、尺寸和明确参考图；batch 可附 negative |
| 角色设定/年龄/表情/姿势差分 | 角色视觉提示词、当前变体要求和身份参考图 |
| 物体素材 | 外观、材质、可见状态、环境和风格 |
| 场景素材 | 空间布局、入口动线、氛围和可见环境状态 |
| 导演图片点 | 当前点静态画面、所属镜头必要信息、绑定场景、用户要求、参考图 |
| 视频首帧/边界帧 | 当前帧文字、negative、单张画面规则和真实图像绑定 |
| 视频分镜板 | 当前单元分镜文字、negative、2–6 格规则和图像绑定 |
| 漫画页 | 当前页 image_prompt 与本页素材绑定 |
| Galgame 背景/CG | 当前场景/事件模板和用户要求；CG 身份充分性见下文 |
| Canvas 图片 | 实际相连的上游文本、节点设置及用户选择 |

图片合同本身没有原生 negative 槽位，本次 batch 的 negative 以自然语言约束合入正向文本。对专门训练的负向编码器，这不等同于原生负向条件；没有擅自给所有端点添加未确认的字段。

| 图片后端 | 实际发送方式 |
|---|---|
| openai-compatible-image | generations prompt/size；参考图不支持时拒绝 |
| openai-compatible-image-edit | 无图 generations；有图 multipart edits，保留参考序号/标签 |
| cliproxy-image | generations 或 multipart edits；尺寸转换为 resolution/aspect_ratio |
| openai-chat-image | user 文本 + image_url 多模态块；尺寸与参考标签保留 |
| stable-diffusion | txt2img prompt、尺寸、采样配置；当前不支持参考图和独立 negative 合同 |
| comfyui | 固定文生图 workflow，正向 prompt、负向空字符串；不支持参考图 |

## 条件性风险

- Galgame CG 主要依赖事件文字和 `cg_note`，没有统一的角色视觉身份/参考图合同；文字已包含外貌时可能足够，但不能承诺所有 CG 与立绘保持一致。本轮未新增资产绑定产品能力。
- Galgame 背景/CG 默认模板写固定 16:9，而请求尺寸可配置；用途与画幅仍需实际产品场景验证。
- 通用画像模板要求设定图/三视图，Galgame 立绘变体是否适用取决于实际生成/复用路径；没有把“可能冲突”写成每次都会错误。
- 上游 LLM 接收的参考槽位与最终 Provider 的帧角色必须一起验证。历史全链并集的推断不能直接当成当前执行 recipe 已错位；保留实际绑定、去重及来源版本检查。
- 实际尾帧来自 ffmpeg，不是一次图片模型调用；多模态的参考媒体不能按 base64 字符数记作普通文本 token。

Provider 构造器和请求级测试覆盖 negative、最终长度和引用顺序；前端三种请求在真实 Chromium 中执行。CLI/Mock 的最终回归状态与局限统一记录于 [README.md](README.md)。
