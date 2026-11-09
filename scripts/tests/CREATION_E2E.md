# 创作阶段端到端验收

运行入口：

```bash
python3 scripts/tests/run_creation_e2e.py
python3 scripts/tests/run_creation_e2e.py --stage inspiration
python3 scripts/tests/run_creation_e2e.py --list
```

`python3 scripts/tests/run_creation_e2e.py --stage all --browser-feature-flows` 是完整 Web 与 CLI 功能链路 Mock 验收入口。它运行下列阶段测试和浏览器流程，并覆盖媒体改编、角色/场景同步、Agent 委派与领域 Agent、LLM 生命周期/缓存、图片/立绘生成与恢复、媒体创作工作区、特化 Agent 工作区、镜头同步浏览器流程，以及 `run_all_workflows_e2e.py` 的全业务 fixture。任一子套件缺失、无断言、超时或失败，均使整轮失败；报告中的 `generation_matrix` 汇总全量 CLI 套件状态。

Nightly Full Checks workflow 会自动运行 `--stage all --browser-feature-flows`；PR 只运行轻量矩阵和核心数据库集成。需要在本地复现同一组测试时使用 `./scripts/test-all.sh --profile full`，不要把完整 E2E 当作只编译或只启动服务的检查。

需要已有 Rust、Node/npm 依赖、Docker、FFmpeg、Chromium/ChromeDriver、项目固定版本的 ONNX Runtime 和 `tools/qdrant/bin/qdrant`。PostgreSQL 默认使用 `postgres:16` 并设置 `max_connections=300`，以容纳全量并行 worker 的连接池；私有镜像地址可通过 `--postgres-image` 或 `A4N_E2E_POSTGRES_IMAGE` 指定。

入口把启动时工作区保存为完整源码快照，持共享构建锁编译，并保存可校验的独立二进制；随后创建自己的 PostgreSQL 容器、Qdrant、Mock Provider、Server/Worker 和浏览器。不会连接已有开发库，也不会改用真实付费模型。`--binaries-dir` 只能复用本工具构建、文件哈希和当前输入快照均匹配的二进制。`--application-source <旧报告/source>` 用于复现保存的应用版本，测试脚本仍取当前工作区；报告分别记录被测后端、前端哈希及是否与当前工作区一致。

浏览器直接访问 Server 提供的生产前端构建，与 API 同源。二进制指纹包括 crate 内资源、ONNX 版本、构建脚本及 Cargo/工具链配置；前端指纹包括公共资源。本轮补齐了旧指纹遗漏的输入，旧二进制回执需要重新构建后才能复用。

## 两端如何界定

“接口返回 200”或“模型回复结束”不是阶段终点。阶段从用户发出操作开始，到正确结果能持久化重读、能供下游使用为止。前端用真实输入、点击、刷新和浏览器媒体解码；CLI 用真实权限、任务/工作流、持久化 API 和 Provider 请求进行补充验证。浏览器不注入登录 token、不替换业务请求、不用 API 写入来冒充页面操作。

| 阶段 | 起点 | 终点与重点 |
| --- | --- | --- |
| `inspiration` 灵感 | 页面输入、选择建议、追加要求 | 回复完成、历史重读、长历史进入模型、重复启动幂等；失败/取消不保存半轮对话 |
| `blueprint` 蓝图 | 从灵感发起四阶段生成或精修 | 任务结果与正式版本一致；角色/世界观可读取；手工编辑、恢复版本及下游大纲消费 |
| `outline` 大纲 | 从当前正式资料生成分部和章纲，编辑计划 | 编号/范围/归属正确；精确保存、拒绝旧版本写入；正文使用修改后的章纲 |
| `chapter` 正文 | 基于章纲开始生成、保存正文 | 正文落库且引用对应章纲；刷新不丢文、旧 revision 不覆盖新文；保存不暗中调用模型 |
| `analysis` 分析 | 对当前已保存正文发起分析 | 引用来自当前原文，正文不被改写，资料同步完成；正文更改后旧事实撤回与重建 |
| `director` 导演 | 用户编辑时间线、绑定场景、确认方案 | 刷新仍一致，确认版本被剧本绑定；点/镜头/素材与时长不混淆，局部修改正确失效 |
| `shooting` 剧本 | 从已确认导演方案生成、继续或编辑 | 可读取已保存场景和最终剧本；断线不中止任务、停止可见、错误不假完成、恢复保留有效产物 |
| `segments` 片段规划 | 对指定已保存拍摄场景规划、编辑 | 片段绑定实际镜头与时长；刷新保留手工编辑；恢复和局部更新不覆盖无关结果 |
| `media` 图片/视频 | 使用当前片段、参考图与提示词发起生成 | 请求实际到达 Provider，图片解码、视频采用、刷新后完整播放；参考图字节、负面约束及导出/导入关系正确 |

跨阶段链路还必须验证“上一步编辑后，下游看见的是新内容”及“上游变化后，旧下游不能继续当成当前结果”。这与每个阶段的独立异常/恢复用例同时构成门禁。

## 复用的检查

- `run_authoring_stage_cli_e2e.py`：灵感、蓝图、规划编辑、正文、分析的结果/版本/下游输入。
- `run_checkpoint_call_cli_e2e.py`：各模型调用的非法输出、有效断点复用及重试调用次数。
- `run_fact_feedback_cli_e2e.py`：正文事实撤回、周期反馈、来源覆盖及项目族往返。
- `run_analysis_anchor_cli_e2e.py`：通过 OpenAI Responses 协议依次注入物品、角色、场景及角色同步的字符串证据 ID，验证明确失败、拒绝无效落库、有效断点复用及最终续跑完成；随 `analysis` 阶段运行，也可连接隔离 Server 单独执行。
- 该套件同时检查实际发送的 JSON Schema，并验证字符串 ID、缺少来源字段和整数但未知来源三类错误经一次有限纠正后可以完成。持续错误最多调用两次，错误不被吞掉；其他故障套件的调用次数断言也应包含这一次明确的纠正。
- `run_director_timeline_cli_e2e.py`：导演、镜头、片段、图片/视频、局部失效与媒体导出。
- `run_shooting_async_cli_e2e.py`：后台任务、观察者断线、取消、部分结果、恢复与来源同步。
- 全量生成矩阵额外运行 `run_media_authoring_cli_e2e.py`、`run_character_sync_resume_cli_e2e.py`、`run_scene_sync_cli_e2e.py`、`run_agent_cli_e2e.py`、`run_agent_domains_cli_e2e.py`、`run_specialized_agent_cli_e2e.py`、`run_llm_lifecycle_cli_e2e.py`、`run_llm_cache_cli_e2e.py`、`run_image_publication_cli_e2e.py`、`run_image_checkpoint_recovery_cli_e2e.py` 和 `run_portrait_generation_cli_e2e.py`。立绘套件覆盖新增、重绘、年龄变体及缺失批量生成，并检查 LLM/图片 Provider、任务终态、结果引用和正式记录。
- `run_specialized_agent_cli_e2e.py` 的空白导演方案携带页面编辑器来源、使用长正文触发结果截断，并注入误用图片点预览工具的参数错误；验证错误提供合同查询入口、按 JSON pointer 读回 draft/confirmed/starter_plan、无在线编辑器响应时仍能审批保存及后续编辑，最终重读真实草稿的镜头、图片绑定、时长和版本。确定性决策只验证这条恢复与持久化路径，不证明真实模型每次都会选对工具。
- 同一套件故意把转场两端场景对调，再通过导演 Agent 调用 `confirm_chapter_visual_plan`，验证真实确认失败会返回镜头路径、起止场景的期望值和实际值；Agent 读取相邻镜头修复后，按 `mode=confirmed` 重验并再次确认。自主模式仍遵守确认审批，未审批不写入，过期版本不会改变确认结果；最终确认资源、其余编排字段和后续剧本投影都实际重读验证。
- `run_llm_lifecycle_cli_e2e.py` 还验证 Canvas LLM 输出驱动图片生成、图片作为视频首帧、Provider 请求与三类节点产物/checkpoint 读回。
- `run_all_workflows_e2e.py` 以隔离 Mock Provider 执行 novel、分析/剧本、资料同步、Galgame、Manga、video 等 fixture；每个 `FIXTURE_PATHS` 业务路径和 chat/image/video Provider 路由都必须通过。
- `generation-stages.e2e.mjs`：由真实页面操作完成生成、编辑、刷新及下游使用。

选择某阶段会运行相关测试组；复合组可能额外覆盖其他阶段。CLI 与浏览器都必须有该阶段的实际检查结果。仅启动服务、空报告、未执行、依赖失败、超时均不能计作通过。测试组进程非零退出时，整轮失败，即使已经保存了某些阶段的通过记录。损坏或缺失的测试组报告明确记为失败，继续检查其他独立组；同一串行创作链的上游失败时，下游标为未验证，不伪造结果继续。

## 与其他端到端测试的关系

| 入口或测试 | 保留的独有覆盖 |
| --- | --- |
| 上述五组 CLI 脚本 | 统一入口的组成部分，也用于快速定位阶段故障 |
| `full-project.e2e.mjs` | 手工建档、导入正文、RAG、Agent、画布、项目族导出 |
| `nonlinear-material-sync.e2e.mjs` | 自由创作、无正文资料、跳过空白章节、归纳候选确认及正式蓝图更新 |
| `video-mode-switch` / `video-scrub` | 多种生成模式、多片段状态、取消、播放与拖动竞态 |
| `package-lifecycle.e2e.mjs` | 管理员启停功能包、普通用户边界及个人隐藏偏好；由完整创作 E2E 一起运行 |
| `tauri-generation/generation.e2e.mjs` | Tauri Native 路径创建项目、流式生成及离开页面后恢复查看；由 macOS `tauri-functional` profile 运行 |

## 用户功能覆盖

| 功能区 | 流程覆盖 | 执行入口 |
| --- | --- | --- |
| 首页与 Agent | 项目筛选、邀请查看、创建/导入入口、线程活动、审批、子线程和预算操作 | `browser-ui-functional`：`H01-H08`、`A01-A12` |
| 设置 | 提示词/主题、模型配置、用户和项目管理、任务队列、账号认证、日志、用量与备份 | `browser-ui-functional`：`SD01-SD27` 及 `S02`、`S05`、`S06` 操作用例 |
| 创作编辑器 | 草稿确认/丢弃、章节导入与数值范围、阶段切换、视频时间输入 | `browser-ui-functional`：`UX-B60`、`UX-B72`、`UX-B73`、`UX-B74`、`UX-698` |
| 正文事实反馈 | 保存归纳周期、执行反馈、关闭进行中的预览 | `browser-fact-feedback`：两个隔离 UI 流程 |
| 视频画布 | 键盘添加节点 | `browser-ui-functional`：`X01` |
| 创作阶段 | 灵感到媒体九个阶段的创建、生成、编辑、刷新重读和下游使用 | `creation-e2e`：`generation-stages.e2e.mjs` |
| 完整项目 | 登录、项目创建、RAG、Agent、衍生项目和项目族导出 | `creation-e2e --browser-feature-flows`：`full-project.e2e.mjs` |
| 功能包 | 管理员启停与用户偏好 | `creation-e2e --browser-feature-flows`：`package-lifecycle.e2e.mjs` |
| CLI | 九个创作阶段，以及媒体、同步、Agent、LLM、图片/立绘与工作流任务 | `creation-e2e --stage all` |
| Tauri Native 生成路径 | 桌面端创建项目、流式生成、离开后返回并读取完成结果 | macOS `tauri-functional` profile |

`browser-ui-functional` 和 `browser-fact-feedback` 在 nightly 与 nightly-quick 执行；Tauri 用例需要设置 `A4N_WDIO_APP_BINARY`、隔离数据库 `A4N_DATABASE_URL`、`A4N_QDRANT_URL` 和 `A4N_JWT_SECRET`，运行命令为 `python3 scripts/tests/run_test_matrix.py --profile tauri-functional`。这些流程只验证用户功能行为，不扩展性能、安全或视觉质量门禁。

原 `resume-existing.e2e.mjs` 的独有断言已并入非线性测试，旧文件退役。现在由同一次运行创建项目、导入第 7 章、完成同步，再验证分部归纳保留稀疏章纲和蓝图正式写入；不再依赖人工留下的项目 ID，也不以自动重试掩盖生成或同步失败。

模型调用登记门禁及显式启用、有预算的真实模型验收入口见 [模型调用可靠性](../../docs/model-call-reliability.zh.md)。完整 Mock 流程继续采用确定性 Provider，真实模型验收单独报告，不混算成功率。

非线性测试可用 `cd frontend && npm run test:browser-full-project -- --spec ./e2e/nonlinear-material-sync/nonlinear-material-sync.e2e.mjs` 运行。需要已启动的隔离 Server，通过 `A4N_BROWSER_E2E_BASE_URL`、`A4N_BROWSER_E2E_USERNAME`、`A4N_BROWSER_E2E_PASSWORD` 指定页面与登录信息，并提供 `A4N_NONLINEAR_LLM_API_KEY`。使用自定义模型时同时指定 `A4N_NONLINEAR_LLM_PROVIDER`、`A4N_NONLINEAR_LLM_BASE_URL`、`A4N_NONLINEAR_LLM_MODEL`，否则使用该 Server 的系统 LLM 配置。它使用独立的本地哈希嵌入配置，模型的故事内容必须遵循测试输入；不能把固定返回另一组人物和情节的 Mock 当作真实模型评测。

## 报告及保证边界

报告保留在 `data/creation-e2e/<run-id>/`，包括 `report.json`、源码快照及二进制哈希、各组检查、Provider 请求、逐阶段截图、失败 DOM、浏览器异常堆栈、运行日志和数据库文件。页面出现严重控制台错误或未恢复的 HTTP 错误，即使业务断言成功也不能通过。生成事件协议用 HTTP 409 表示需要快照；仅当同一任务、同一 attempt 的后续快照返回 200 且阶段业务断言成功时，才记录为已恢复协商，证据另存 `*-recoveries.json`。日志读取失败、资源清理失败也会报错；失败清理继续处理其他资源。成功或失败后均停止自己启动的服务；证据保留。输入版本改变后必须重新验证，旧版本结果不能转移到新代码。

通过表示这些明确列出的用户行为、数据不变量和故障场景在该版本上通过，不能推出“不存在任何 bug”。Mock 验证编排、状态和协议，不证明真实模型的文学/导演质量、所有厂商行为、所有历史数据升级或 Native/Tauri IPC。上述方面需要相应真实模型评测、数据迁移样本和已有 Native 测试，不能由 Web/Mock 的绿色结果代替。
