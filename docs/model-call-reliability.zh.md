# 模型调用的合同、纠正与验收

这套保障把模型输出视为外部输入：先完整校验，再采用结果、写入业务记录和完成断点。Mock 流程通过、Provider 连通、Schema 已发送、真实模型稳定和内容质量，是不同的验收结论。

## 生成与解析合同

- Host 和 LLM sidecar 共用 `LlmResponseFormat`，支持 `json_object` 及包含 `name/schema/strict` 的 `json_schema`，持久快照和 sidecar 转换保留全部字段。
- 蓝图四阶段从既有输出形状合同投影 JSON Schema；角色、物品、场景及角色同步从同一份 `field_types` 和必需字段合同投影 Schema。正整数、来源 ID 数组、重要性和物品/场景枚举复用已有领域规则。
- 角色属性和场景状态中的动态字典保留业务允许的结构，因此这些任务不强制 `strict=true`。来源候选、章节范围、身份冲突、引用顺序、长度与跨字段规则仍由领域 validator 检查，Schema 不替代这些规则。
- `strict=false` 不能保证模型遵循 `required`。场景任务的文字合同会从同一份 Schema 派生 `item_required_fields`；进入/离开状态即使为空也须由模型显式输出对象，缺字段、`null` 或其他类型继续拒绝，不由应用自动补值。
- 原生 Schema 也计入运行时和预览的上下文预算；使用提示词 Schema 时，适配发生在预算选取之前。

| Provider 预设 | Schema 的传递方式 |
| --- | --- |
| `openai` | `response_format.json_schema` |
| `openai-responses`、`custom-responses` | `text.format` 的 `name/schema/strict` |
| `gemini`、`custom-gemini` | `generationConfig.responseJsonSchema` |
| `anthropic` 且显式 `strict=true` | `output_config.format` |
| 其他预设，包括普通 Anthropic、DeepSeek 和自定义 Chat Completions | 声明为提示词 Schema 模式，保留 JSON 模式和应用校验；不能承诺原生严格约束 |

这是一项确定的协议适配，不会在 API 报错后自动换协议或去掉约束。未声明原生严格能力的预设收到 `strict=true` 时明确拒绝；上游不支持其协议预设所要求的 Schema 时返回实际错误。新适配应先完成协议与真实模型验收，不以静默降级掩盖不支持的能力。

LLM capability 已升级为 **`llm-chat-v9`**，Host 和 LLM gateway 必须同步更新并重启。版本来自共享协议常量；进程目录发布对应版本。旧 gateway 不能被视为具有新 Schema 能力，旧持久快照中没有 Schema 的请求仍可解析。

## 有上限的输出纠正

HTTP 连接阶段的临时故障与模型输出纠正分开处理。Chat Completions、Responses、Gemini 和 Anthropic 共用传输发送入口；只有 reqwest 明确标记为连接建立失败、尚未发送模型 HTTP 请求的错误，才按 100 / 300 毫秒间隔最多重试两次。连接超时直接失败，证书验证和原请求内容、鉴权、输出额度始终保留。请求已发送后的 EOF、响应读取失败、HTTP 错误、模型拒绝和合同错误不会由此重发；取消会同时终止等待中的连接重试。

这只能恢复短暂连接波动，不能保证外部供应商持续可用。漫画分页遇到持续失败仍明确报告对应页，保留已完成的页纲与单页检查点，续跑只补失败页。`generation SSE replay floor passed` 表示客户端需要重新获取任务快照，与模型 Provider 连接故障分别处理。

结构化业务入口复用既有完整 validator。首次输出不合约时，最多进行 **一次额外调用**：保留原任务、来源及配置，追加明确校验错误与受限的上次响应摘录，要求重新生成完整结果，再执行同一个 validator。

存在字段 Schema 的请求会在纠正反馈中再次附带原始 `output_schema`，让模型直接检查 `required` 和字段类型；它与原生协议使用同一个合同，也参与新增消息的上下文预算。纠正仍可能失败，Mock 通过不能证明外部模型必然遵循合同。

- 纠正复用首次调用的模型配置及统一流驱动：连接与无新数据超时由模型配置决定，单次调用仍受共享总时限约束；不另设更短的业务超时，不提高既有输出 token 上限，不切换模型或供应商。
- 错误说明最多 2,000 字符，响应摘录最多 8,000 字符；长响应同时保留开头与末尾，避免尾部非法内容被截掉。摘录及省略标记作为不可信数据传递，调用诊断按已有日志和 trace 配置保留。
- 原始 JSON 在字段 Schema 和业务校验之前拒绝重复字段、尾随内容、代码围栏和语法损坏。同一对象的重复字段即使值相同也拒绝；不同对象各自拥有 `weather` 等同名字段合法。重复字段错误带 JSON Pointer，解析错误另附最多 1,200 字符的报错位置摘录，覆盖长响应中段与中文多字节内容；纠正明确要求在该对象中只生成一个符合来源和合同的值，不静默取首值或末值。
- 配置、权限、状态失效、取消、传输失败、模型拒绝和输出触顶不进入盲目重复生成。没有完整响应的失败继续显式返回。
- Agent 工具决策回合使用原有迭代与预算规则，不在外面额外增加不计预算的模型回合。
- 两次输出都不合法时，错误包含首次校验和最终失败，保留已有工作流恢复能力；不进行字符串 ID 强转，不删除必需字段，不返回空结果来假装成功。

已接入共享纠正的入口包括灵感、蓝图、分部/章节大纲、章节分析、角色/物品/场景同步、隐性属性分类/复审、角色冲突、导入反推、漫画、Galgame、拍摄剧本、视频结构化规划/优化、局部正文补丁和人物提示词。普通正文与视觉描述仍使用原有文本完成、取消和断点机制。

漫画的“页数与页面规划”读取已采用改编计划的页数，先调用一次 `manga_page_outline` 分配轻量页纲，再为每页独立调用 `manga_page_content`，正常共 `1+N` 次文本调用，不包括预处理、章节改编及后续视觉阶段。页纲统一持有来源、节拍和边界状态，程序组装现有 `MangaPageSlotsV1`；单页模型只生成页码、摘要、动作、反应与对白意图。每次调用独立校验、纠正和保存检查点，最多两页并发且服从模型共享并发限制。失败后停止派发新页并保留在途成功检查点，全章完成前不发布残缺分页；恢复只补缺失或失效的调用。该拆分不保证总耗时或费用下降，也不替代对 Provider 断流、服务容量及生成内容质量的独立验证。

分页执行与请求预览在合并已保存的单页内容后，共用完整的页数、来源引用、关键节拍和衔接校验。漫画分镜的文字位置直接使用领域类型生成的八个规范值，提示词、请求 Schema、解析和持久化合同一致；`upper_right` 等别名须触发输出纠正，不能在程序中偷偷改写为规范值。

已保存的分页按产物过期状态和当前页数、来源、节拍合同复用；模型请求指纹只约束未发布调用的检查点，不能阻断有效的人工分页或历史产物。删除分页或预处理后，保留页面投影回到页级 `content/pending` 状态，写入前重新校验布局。`manga_page_slots_reset_repair` 迁移修复旧删除路径留下的 `slots/pending` 页面，保留页面身份、内容、绑定和图片；章节级 `slots` 不作为合法页面状态放宽读取。

生成事件的文本流数量上限为 1,024，事件上限为 256 KiB，容纳百页章节的页级阶段及纠正流哈希；浏览器字节限制从 Rust 常量生成。单段文本 16 KiB、快照 4 MiB 和运行时全局内存/活跃会话限制继续生效，超出容量仍明确失败。

纠正调用的 trace key 带 `.correction.1`，使用独立指纹，并重新规划新增反馈的上下文预算。两次用量均已知时合并当前逻辑调用的用量；任一次未知则当前调用总用量置空，不补零。任务此前的已知用量保持不变，原始 API 调用仍各自计量，费用事实以逐次调用记录为准。

系统输出约束也属于请求指纹。更新约束后，不再匹配的旧调用检查点会按既有规则失效，未完成任务可能重新调用对应步骤。

结构化 generation 任务在收集候选时继续发布活体进度和 reasoning，最终采用流 `raw_json` 只发布校验后的结果，避免无效响应混入最终文本、哈希和断点。候选的原始文本通过既有 Provider trace 排查；正文等自由文本任务继续原有增量显示。

## 确定性回归与 CI

| 检查 | 守住的边界 |
| --- | --- |
| 共享纠正单测 | 正常结果零额外调用、最多一次纠正、取消、截断、状态错误、工具回合、错误保留和用量合并 |
| 生产入口原始 JSON 矩阵 | 每个共享结构化调用分别注入重复字段、尾随 JSON、代码围栏和损坏 JSON，检查有效替换与持续拒绝。原文不先转为对象，防止重复字段被解析器覆盖；这些必测项缺失时报告验收失败 |
| 纠正时间与长输出回归 | 虚拟时钟下 120 秒纠正可完成；长 JSON 的尾部多余内容仍被拒绝，8,000 字符反馈保留出错尾部；真实 Responses Mock CLI 仅纠正一次，校验后才持久化和采用改编计划 |
| generation 流测试 | 尝试内容与采用内容分离，最终文本与哈希一致 |
| Schema/协议测试 | 输出合同不会在持久快照、Host/sidecar 或原生协议中丢失；不支持的严格请求显式失败 |
| `run_analysis_anchor_cli_e2e.py` | 原生 Responses 下九类非法证据 ID 的有限失败、拒绝无效写入、有效断点复用；字符串 ID、缺少来源字段和整数但未知来源三类输出可在一次纠正后完成 |
| `run_scene_sync_cli_e2e.py` | Chat Completions/Responses 下状态字段持续缺失时有限失败、不运行下游或写入无效结果；模型在一次纠正中补齐 `exit_state` 后才采用结果和保存断点 |
| `run_media_authoring_cli_e2e.py` | Chat Completions/Responses 下视觉预处理原始重复 `weather` 的纠正和持续失败；Mock 必须收到字段路径、定位原文和唯一字段规则才能修正。两次都非法时不发布预处理产物或运行综合，续跑只补视觉与综合，复用有效的事实和对白 |
| `run_checkpoint_call_cli_e2e.py` 等既有业务套件 | 蓝图、剧本、漫画、Galgame、优化等错误与阶段恢复；完整创作入口继续运行这些套件 |
| `check_model_call_coverage.py` | 新增/删除已知聊天 driver 调用、调用次数变化、未登记恢复策略和失效测试入口使门禁失败 |

`model_call_coverage.json` 是调用登记表，包含业务入口及共享执行器，不是运行覆盖率，也不与输入/输出清单中的业务调用数相加。静态检查不替代执行测试，不把“文件存在”写成“业务已通过”。运行断言以每次 CLI/E2E 报告为准。

登记门禁已接入 `check_test_catalog.py`，随 fast/nightly profile 执行；`model-contracts.yml` 在相关 debug/master/main push 和 PR 上运行基础合同与有限纠正测试。完整业务 Mock 验收继续由既有 creation E2E job 执行。CI 不会默认调用付费模型。

## 真实模型业务验收

`run_live_model_eval.py` 使用真实 CLI、Server 和业务 validator，对三个大小的虚构材料进行独立重复试验。每个样本创建独立测试项目，验证分析和角色/物品/场景同步、正文保持及非空实体结果。它不会给创作质量打分，也不把这几个场景的结果外推到所有业务或模型。

必须连接使用一次性数据库、独立端口和数据目录的 Server，并提供已有的本地 Mock Provider 作为 Embedding 测试依赖。普通创作数据库不可用于此入口；配置快照只保存在本次临时 CLI 目录，不改日常模型配置。

无凭证配置文件示例：

```json
{"provider":"openai-responses","base_url":"https://api.openai.com/v1","model":"待测模型名称","max_context_tokens":128000,"timeout_secs":60}
```

先设置登录密码和模型凭证环境变量，再执行：

```bash
python3 scripts/tests/run_live_model_eval.py \
  --a4n target/debug/a4n --server http://127.0.0.1:18080 \
  --username model-eval --password-env A4N_EVAL_PASSWORD \
  --provider-url http://127.0.0.1:18081/v1 \
  --provider-log data/model-eval/provider.jsonl \
  --llm-config data/model-eval/model.json --api-key-env A4N_EVAL_API_KEY \
  --samples 3 --material baseline \
  --max-calls 120 --max-output-tokens 8192 --timeout-seconds 1800 \
  --isolated-server --report data/model-eval/result.json
```

不指定 `--material` 时依次测试 baseline/extended/long；`--samples` 是每份材料的独立重复次数。开始前按模型价格选择预算。relay 在实际出站前限制调用数、每次输出上限、请求大小和总时间；额度用尽后未执行样本明确计为失败，不从成功率分母移除。模型密钥只来自指定环境变量，不写入配置快照或报告。

报告记录每个样本的结果、纠正调用、耗时、Provider 原始用量，以及首次完成率和校验后完成率。缺凭证、空报告、失败、未执行或超预算都不能通过。`--allow-local-http` 仅允许 loopback 测试；此时报告标为 `fixture_runner_validation`，不能当作真实模型成绩。

Mock 用于可复现地证明程序行为；真实模型验收用于判断选定 Provider/模型在这些材料上的适用性。仍需对实际作品进行内容质量与长期连续性验收。

协议依据：[OpenAI Structured Outputs](https://developers.openai.com/api/docs/guides/structured-outputs)、[Anthropic Structured Outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs)、[Gemini Structured Output](https://ai.google.dev/gemini-api/docs/structured-output)。
