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

现有 `provider` ID 保持原行为，无数据库迁移。新增目录字段与可选工具上下文需要配套更新前端、Runtime 和 LLM Gateway。

## 验证

按 AGENTS.md 在 agent 工作树、共享构建锁及指定 target 下运行：

```bash
timeout 60s cargo test --locked --offline -p a4n-domain -p a4n-llm -p a4n-llm-http -p a4n-llm-config -p a4n-model-discovery --lib
timeout 60s python3 scripts/tests/test_mock_openai_provider.py
```

前端运行 `npm run test:run -- src/pages/settings/llm/llmConfigUtils.test.ts`、`npm run build` 和 `npm run test:browser-llm-presets`。浏览器检查使用本机 Chromium/ChromeDriver，覆盖预设、换钥、模型发现、空结果、错误、键盘及 1440/1024/390px 布局。

CLI 使用现有 `run_cli_contract_e2e.py`，指定 `--ledger scripts/tests/cli_contract_llm_protocols.json`，分别传入 `--llm-provider openai`、`openai-responses`、`anthropic`、`gemini`。测试 Host 必须安装 AI 工作流包；该清单通过公开生命周期 API 启用此包，需在隔离测试环境传入 `--allow-destructive`，不应指向用户日常数据库。运行时配置快照使用当前 v2 合同。其余 Server、登录及报告参数参见 [虚拟 Provider CLI E2E](virtual-provider-cli-e2e.md)。

## 官方依据

- [OpenAI Responses](https://developers.openai.com/api/docs/guides/migrate-to-responses)、[工具调用](https://developers.openai.com/api/docs/guides/function-calling)
- [Gemini GenerateContent](https://ai.google.dev/api/generate-content)
- [DeepSeek](https://api-docs.deepseek.com)、[通义兼容接口](https://help.aliyun.com/zh/model-studio/compatibility-of-openai-with-dashscope)
- [Kimi](https://platform.moonshot.cn/docs/api/chat)、[智谱](https://docs.bigmodel.cn/cn/guide/develop/openai/introduction)、[火山方舟](https://www.volcengine.com/docs/82379/1330626)
- [硅基流动](https://docs.siliconflow.cn/cn/userguide/quickstart)
