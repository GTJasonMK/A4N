# LLM 协议与厂商预设

文本模型调用统一使用 `ChatRequest → LlmProvider::chat_stream → ChatChunk`。
厂商预设定义在 `crates/a4n-domain/src/llm/providers.rs`；配置校验、请求适配、模型发现及设置页共用此目录。

## 支持范围

| 协议 | 请求端点 | 支持内容 |
| --- | --- | --- |
| OpenAI Chat Completions | `/chat/completions` | 文本流、JSON、工具调用、用量 |
| OpenAI Responses | `/responses` | 文本流、JSON、工具调用、reasoning、用量 |
| Anthropic Messages | `/v1/messages` | 复用现有文本流、工具调用和用量适配；JSON 使用现有提示词约束 |
| Gemini GenerateContent | `/models/{model}:streamGenerateContent?alt=sse` | 文本流、JSON、工具调用、thought、用量 |

预设包括 OpenAI（两种协议）、Anthropic、Google Gemini、DeepSeek、通义千问、Kimi、智谱、豆包、硅基流动、Ollama，以及四种协议的自定义入口。
预设提供协议和默认地址；模型由用户填写或通过“检测”读取，避免把模型版本写死。

选择厂商后填写对应 API Key、选择模型并测试连接。切换预设会清空旧模型与密钥；修改已有配置时，旧密钥在保存时清除，可直接输入新密钥替换。已手动修改的代理地址会保留。
上下文窗口和最大输出须按实际模型设置；预设不代表同一厂商所有模型都支持相同能力。

自定义地址填写 API 基础路径：Chat/Responses 通常包含 `/v1`，Anthropic 不含适配器自动追加的 `/v1`，Gemini 通常包含 `/v1beta`。
地域、业务空间和代理路径以提供方配置为准。通义预设保留仍可使用的公共域名，业务空间专属域名可直接覆盖。

## 实现约束

- 新增兼容协议厂商只追加目录项；新增协议才增加 payload 和流事件转换。
- Responses 使用 `store: false`，工具调用携带加密 reasoning 项；Gemini 保留 thought signature。`ToolCall.provider_metadata` 会穿过流聚合、Sidecar 和持久化序列化，重放时恢复协议上下文。
- 新工具消息的幂等标识使用调用内容的 SHA-256，避免把大段参数或加密上下文直接放入数据库索引。
- Responses 不支持 `stop_sequences`，明确返回错误。高级参数不能覆盖系统管理的消息、输出预算、工具等字段；Gemini 的 `generationConfig` 可设置非系统字段，例如 `thinkingConfig`。
- 缺少协议终态、无效 JSON、上游错误和受限输出保持显式失败；截断保留原始结束原因，交由现有生成执行层处理。
- 本轮支持文本及函数工具调用，不增加音频、实时会话或图片输入合同。厂商预设与协议支持不能替代具体模型的能力确认。

现有 `provider` ID 保持原行为。厂商目录、工具上下文与缓存合同需要配套更新前端、Runtime 和 LLM Gateway；缓存用量迁移见下节。

## 提示词缓存与用量

- 请求层统一声明消息内的 UTF-8 字节边界；Chat Completions、Responses 和 Anthropic 分别转换为原生缓存标记，保留系统规则、共享上下文和历史观察的分层前缀。最多使用上游允许的四次缓存写入，不填充无用文本凑缓存门槛。
- OpenAI GPT-5.6 及后续命名版本在已声明稳定用户前缀、且没有 assistant/tool 历史时默认使用 `explicit`，避免缓存一次性尾部。未声明该前缀的原生对话从首轮保留隐式缓存；Claude 首轮标记完整用户任务。已有稳定用户前缀的漫画、优化等请求仍只缓存声明的边界。旧版 OpenAI 与其他兼容厂商继续使用其自动缓存。
- OpenAI 代理模型使用自定义别名时，可通过高级参数 `{"prompt_cache_options":{"mode":"explicit"}}` 声明显式缓存能力。TTL、诊断等原生选项继续透传；已知不支持该选项的旧模型、非 OpenAI 厂商与非法 mode 在配置保存及发送前共用同一校验。切换到另一协议的预设会清空高级参数；同协议切换保留这些参数。
- 路由键按项目和模型稳定分组，不绑定整个上下文哈希；缓存是否匹配仍由厂商核对实际输入。漫画保留完整输入与输出契约，将本页内容置于动态尾部；优化循环按顺序追加完整观察，历史仍计入上下文预算。
- 用量统一为：输入 = 命中 + 未命中，缓存写入是未命中的一部分。`prompt_cache_write_tokens` 未返回时为 `null`，不推断成零；Claude 的输入总量包含普通输入、缓存读取和缓存写入。统计页展示写入量，缺少缓存数据时不显示推测命中率。
- `llm_cache_usage` 迁移新增写入量列，并只修复可由输入与命中量推导的历史未命中量。未知历史写入量保持空值。该缓存合同使用 `llm-chat-v7`，Host、LLM Gateway 与前端需配套更新。
- `claude_cache_usage_repair` 追加修复前项迁移之前的原生 Claude 记录：将旧未命中列中的写入量移到写入列，并重算输入、总量、未命中及比例；已存在写入量或升级后创建的记录不重算。若旧字段缺失，或无法区分原始写入量与前项迁移推导的未命中量，则相关统计置为 `null`，保留已知命中和输出量，聚合命中率也保持未知。该数据修复不可逆。

缓存回归使用 `scripts/tests/run_llm_cache_cli_e2e.py`，参数与生命周期 runner 一致，需提供专用数据库、已启动的 LLM/Prompt/Video sidecar 和 mock Provider；它通过真实 Canvas 工作流检查四协议断点与持久化用量。计数由 fixture 固定提供，不代表真实厂商命中率。统计页浏览器验证入口为 `npm run test:browser-llm-cache`。

## 验证

按 AGENTS.md 在 agent 工作树、共享构建锁及指定 target 下运行：

```bash
timeout 60s cargo test --locked --offline -p a4n-domain -p a4n-llm-http -p a4n-llm-config -p a4n-model-discovery --lib
timeout 60s python3 scripts/tests/test_mock_openai_provider.py
```

前端运行 `npm run test:run -- src/pages/settings/llm/llmConfigUtils.test.ts`、`npm run build` 和 `npm run test:browser-llm-presets`。浏览器检查使用本机 Chromium/ChromeDriver，覆盖预设、换钥、模型发现、空结果、错误、键盘及 1440/1024/390px 布局。

CLI 使用现有 `run_cli_contract_e2e.py`，指定 `--ledger scripts/tests/cli_contract_llm_protocols.json`，分别传入 `--llm-provider openai`、`openai-responses`、`anthropic`、`gemini`。测试 Host 必须安装 AI 工作流包；该清单通过公开生命周期 API 启用此包，需在隔离测试环境传入 `--allow-destructive`，不应指向用户日常数据库。运行时配置快照使用当前 v2 合同。其余 Server、登录及报告参数参见 [虚拟 Provider CLI E2E](virtual-provider-cli-e2e.md)。

## 官方依据

- [OpenAI Responses](https://developers.openai.com/api/docs/guides/migrate-to-responses)、[工具调用](https://developers.openai.com/api/docs/guides/function-calling)、[提示词缓存](https://developers.openai.com/api/docs/guides/prompt-caching)
- [Gemini GenerateContent](https://ai.google.dev/api/generate-content)
- [DeepSeek](https://api-docs.deepseek.com)、[通义兼容接口](https://help.aliyun.com/zh/model-studio/compatibility-of-openai-with-dashscope)
- [Kimi](https://platform.moonshot.cn/docs/api/chat)、[智谱](https://docs.bigmodel.cn/cn/guide/develop/openai/introduction)、[火山方舟](https://www.volcengine.com/docs/82379/1330626)
- [硅基流动](https://docs.siliconflow.cn/cn/userguide/quickstart)
