# A4N CLI 用户操作流程端到端测试报告

## 结论

- 验收依据：`用户操作流程.txt`，验收时间为 2026-07-29 至 2026-07-30。
- 真实链路：`target/debug/a4n` -> 独立 `a4n-server` -> `a4n-runtime` -> `a4n-orchestrator`/sidecar -> PostgreSQL、Qdrant、文件系统。
- 文档中的 21 组后端业务流程均已通过真实 CLI 验收；最终隔离库中的非终态 workflow 和 generation task 都为 `0`。
- 页面跳转、浏览器本地草稿、系统剪贴板和纯前端预览交互标记为 `不适用（UI）`；对应的服务端保存、检索、生成、导入和下载能力已单独验收。
- 本地注册、密码登录、刷新/撤销 session 已通过。GitHub/Google OAuth 因隔离环境未配置第三方凭据，只验证了“未配置时明确失败”的配置门禁，未伪称完成真实第三方回调。
- 测试及后续收尾共发现 12 个共享根因并完成代码修复；修复后真实媒体、数据库状态、ZIP/EPUB/PDF 结构、CLI 本地生命周期及定向回归均通过。
- 高级 CLI 已覆盖前端使用的全部 381 个 Host API 命令，并提供 Agent envelope、固定退出码、能力 manifest、持久化 context、Shell、可恢复 Flow、危险操作双授权和本地 Server 生命周期管理。

## 高级 CLI / Agent 能力增量验收（2026-07-31）

| 能力 | 验收证据 | 结果 |
|---|---|---|
| 三端命令覆盖 | 前端 `apiInvoke` 381、共享 manifest 381、Web/CLI 缺失与过期映射均为 0 | 通过 |
| Agent 输出 | 成功 stdout/error stderr 均为稳定 JSON envelope；错误含 code/kind/retryable/status/trace/details | 通过 |
| 固定退出码 | input/auth/permission/not-found/conflict/unavailable/failed/cancelled/confirmation/compatibility | 通过 |
| 非交互约束 | `--agent` 禁止 Shell 和隐式输入；密码、OAuth code 只从 stdin 读取 | 通过 |
| 危险操作 | 删除、取消、回滚、重置、恢复覆盖、批量应用及成片后处理必须显式授权；Flow 使用双授权 | 通过 |
| 文件安全 | 原子保存、默认拒绝覆盖、失败下载保留旧文件 | 通过 |
| context | Server/用户绑定，项目/章节/快照层级清理，`userId`/`callerUserId` 等按 manifest 注入 | 通过 |
| task/workflow | JSONL/SSE/轮询观察；failed 返回 8，cancelled 返回 9 | 通过 |
| Flow | source SHA 校验、0600 journal、capture、256 KiB 截断、秘密脱敏、幂等 resume 门禁 | 通过 |
| 仓库外执行 | subprocess 从临时目录运行 capabilities、Flow、下载和安全门禁 | 通过 |
| Local 生命周期 | 隔离资源下分别验证 core-only、8 个 managed sidecar + 外部 Qdrant、CLI-owned Qdrant、embedded/full；均执行 migration、health、381 命令 compatibility、status 和 graceful stop | 通过 |
| 发布包工具 | Linux/Windows portable 与 macOS bundle 均纳入 `a4n`、`a4n-server`、`a4n-migration` manifest/checksum | 通过（结构与脚本） |

发布 smoke 已接入 Linux、Windows、macOS 现有 CI：从仓库外执行 capabilities，随后用
发布包内 migration、Server 和 8 个 managed sidecar 执行 local start/health/
capabilities check/local stop。当前本机没有 PowerShell，且未重新构建三平台完整 archive，
因此不把脚本接入误写成三平台真实生命周期均已在本机执行。

## 测试环境

| 项目 | 值 |
|---|---|
| Server | `http://127.0.0.1:18080` |
| CLI 配置 | 隔离配置，登录用户 `cli_admin`（user `#1`，admin） |
| 主数据库 | `a4n_cli_flow_20260730_043610` |
| 测试数据库 | `a4n_cli_flow_20260730_043610_tests` |
| Qdrant | collection 前缀严格隔离为 `a4n_cli_flow_20260730_043610` |
| AI Provider | 仅监听回环地址的确定性 OpenAI-compatible 测试 Provider |
| 文本主项目 | `#5 逆行时钟`，章节 `#9/#10` |
| 派生项目 | 视频 `#6`（snapshot `#1`）、漫画 `#7`（snapshot `#2`）、GALGame `#8`（snapshot `#3`） |

测试 Provider 不调用计费服务，但保留生产 Prompt、LLM gateway、sidecar、generation task、workflow、数据库和文件持久化链路。报告不记录 token、JWT secret、数据库密码或 Provider key。

## 状态定义

- `通过`：真实 CLI 已执行，并核对任务终态、数据库记录或最终产物。
- `通过（门禁）`：外部系统未配置，但失败合同、错误信息和安全边界符合预期。
- `不适用（UI）`：无前端 CLI 不负责的浏览器/桌面壳交互。

## 逐项验收矩阵

| 文档行 | 用户操作 | 真实 CLI/Host API 入口 | 验收结果 | 状态 |
|---:|---|---|---|---|
| 5 | 注册、登录、session、GitHub/Google 登录 | `auth register/login/logout`、`refresh_token`、`list_sessions`、`oauth2_authorize_url` | 本地用户、未撤销且有过期时间的 session 已落库；OAuth 未配置时明确报配置错误 | 通过；OAuth 为通过（门禁） |
| 7-14 | 项目列表、AI 文本、空白文本、项目包导入、自由视频入口 | `list_projects`、`create_project`、`import_project_*`、`create_freeform_video_project` | 项目类型、所有权、来源关系和画布记录正确 | 通过 |
| 19-23 | 创建文本项目、灵感对话、首版蓝图 | `create_project`、`start_inspiration_generation`、`start_blueprint_generation`、`task watch` | workflow `#1/#2` completed；对话、蓝图版本和项目标题持久化 | 通过 |
| 25-29 | 优化蓝图、世界观、角色、物体、场景、关系与事件 | `start_blueprint_refinement`、`update_world_setting`、profile/object/scene CRUD | workflow `#3` completed；蓝图新版本及实体增删改可回读 | 通过 |
| 31-35 | 分部大纲、章纲生成/续写/编辑/重生成/截断 | `start_outline_generation`、`update_chapter_outline`、`delete_latest_*` | part/outline workflow `#4-#11` 收敛；当前章纲 `#7/#8/#9/#10/#13/#14`，章节号连续 | 通过 |
| 37-39 | 正文生成、手写保存、RAG、Agent 正文优化 | `start_chapter_generation`、`update_chapter_content`、`rag_enhanced_search`、`start_optimize_*` | chapter workflow `#12-#14`、优化 workflow `#15/#16` completed；正文与补丁持久化 | 通过 |
| 41 | 章节结构化分析与派生实体同步 | `generate_chapter_analysis`、`discover_character_profiles`、`sync_scene_profiles`、`sync_character_from_chapters` | workflow `#17/#19/#21/#22/#24-#34` completed；故障注入产生的 superseded 记录不再活动 | 通过 |
| 43 | 角色立绘、场景图、物体图、参考图 | `start_portrait_generation`、`start_image_generation`、`upload_image` | 正式 generation task completed；新立绘 `generated_image #25/#26` 均为真实 640x360 PNG | 通过 |
| 45-49 | 生成和编辑结构化拍摄剧本、继续下一章 | `generate_chapter_shooting_script`、`edit_chapter_shooting_script`、`get_chapter_shooting_script` | 当前记录 `#2`、revision `1`；正文 SHA 与来源一致，剧本 SHA 可复算 | 通过 |
| 54-60 | 空白文本项目、手写/导入并接入主流程 | `create_project`、`import_chapter`、`update_chapter_content` | 空白项目和章节正文可创建、保存和回读 | 通过 |
| 62-82 | TXT/Markdown/EPUB/DOCX 解析、审核、分析、接受/拒绝/删除、去重、结构反推 | import/review/analysis/reverse-engineer 命令 | 4 个格式各解析 2 章；8 个最终审核项 accepted；拒绝/删除分支、项目分析、去重和结构反推均执行 | 通过 |
| 85-97 | 视频/漫画/GALGame 派生项目前置 | `create_derived_project`、snapshot 与 shooting-script 查询 | 项目 `#6/#7/#8` 分别绑定 root `#5` 和独立 snapshot `#1/#2/#3`；来源正文/剧本 revision 匹配 | 通过 |
| 100-126 | 派生视频：素材、规划、五类方案、提示词、生成/导入/采用、合成、推送、后处理 | video prompt/media/adopt/assembly/postprocess 命令 | boundary/first-frame/storyboard/reference-media/Chat 均完成；成片、缓存、插入、删除、替换均为有效视频 | 通过 |
| 129-147 | 漫画流水线、角色绑定、提示词编辑/优化、参考图、逐页生图 | `start_manga_pipeline`、`bind_manga_reference_image`、`update/optimize_manga_prompt`、`start_image_generation` | workflow `#59` completed；实现语义为“确定性拍摄剧本投影 + 7 个 LLM 阶段”，不是 8 次 LLM；页图 `#19` 有效 | 通过 |
| 150-166 | GALGame 7 阶段、脚本编辑、选择支、视觉/音频绑定、WebGAL 导出 | galgame pipeline/edit/media/export 命令 | workflow `#66` 为 `7/7 completed`；脚本 `#1 ready`；3 场、9 行、2 选择项；ZIP 引用闭合 | 通过 |
| 169-191 | 自由视频画布：节点、连线、保存、预览请求、局部/全图运行、输出和包回导 | canvas graph/media/node/run/workflow/export/import 命令 | 文本/图片/视频/音频节点及处理节点均持久化；任务终态和输出媒体已核对；测试项目完成回导后按管理流程删除 | 通过 |
| 194-210 | 来源同步、差异、安全更新、冲突决策、重新生成、detach | source sync/decision/detach 命令 | 安全更新、保留当前内容、GALGame 重建场和 detach 固定快照均完成 | 通过 |
| 213-229 | TXT/Markdown/EPUB/PDF/批量、JSON、项目族/派生/画布包导出 | export 与 package transfer 命令 | 文本内容、magic bytes、ZIP/EPUB 结构、媒体清单和回导结果均核对 | 通过 |
| 232-252 | 项目查看/复制/分支/历史/依赖/删除与协作邀请全生命周期 | project/collaboration 命令 | 复制、分支、状态前进/回退、依赖图、邀请接受/拒绝/撤销、改角色、移除/离开和权限拒绝均通过 | 通过 |
| 255-269 | LLM/Embedding/Image/Video、Prompt、主题、备份、队列/日志/账户信息 | config/prompt/theme/admin 命令 | CRUD、默认项、连通测试、权限门禁、配置备份回导和日志读取通过 | 通过 |
| 271-275 | RAG 全量索引、检索、重建和隔离清理 | `rag_ingest_all`、`rag_enhanced_search` | Qdrant 实际写入并命中正文；重建与隔离 collection 删除通过 | 通过 |

## 关键持久化证据

### 文本主链

- 项目 `#5` 状态为 `writing`；章节 `#9/#10` 均存在，当前正文可导出和检索。
- 当前章节 `#9` 正文 SHA-256：`c656217db2de023754639b19acbaf165d0fc1a2a654071b3e302b1cdfcd3eb4a`。
- 当前拍摄剧本 `chapter_shooting_script #2`：revision `1`，workflow `#41 completed`，script SHA-256：`8afc8a57aa937e20df72eb6277861dbd02d51a888787fd01374ed6f3e1f750df`。
- 生成、优化、分析和同步的最终 workflow 均为 terminal；历史中的 `failed/superseded` 是边界验证和修复前重试记录，不是最终活动任务。

### 四格式导入

| 格式 | 项目 | 审核项 | 最终章节 |
|---|---:|---:|---:|
| TXT | `#1` | 2 accepted | 2 imported |
| Markdown | `#2` | 2 accepted | 2 imported |
| EPUB | `#3` | 2 accepted | 2 imported |
| DOCX | `#4` | 2 accepted | 2 imported |

真实输入文件分别由 TXT、Markdown、合法 EPUB ZIP 和合法 DOCX/OOXML 生成；不是仅修改扩展名。审核队列最终共 8 条 accepted，章节标题和正文内容可回读。

### 视频

- OpenAI-compatible Base64 回归任务：task `93c95f0f-4a3c-4c13-8797-311e124558d3`，attempt `1`，status `completed`，result `generated_video #20`。
- 视频 `#20`：project `#6`、chapter `#9`、storyboard、H.264 640x360 24fps、AAC 48kHz 单声道、6.000 秒、1,354,764 bytes。
- 文件、task result revision 与 artifact metadata 的 SHA-256 一致：`ddeeb1291e83eb5f94c1c06f78854761bdb9305650d59da670227d0bbbab027d`。
- `provider_response` 只保留 `{provider_job_id, artifact:{format,byte_size,sha256}}`，没有媒体 Base64 键或大字符串。完整 metadata 中的字面量 `b64_json` 仅位于 `provider_request_snapshot.request.request.response_format`，表示请求的响应格式，不是返回媒体；metadata 不存在超过 100KB 的字符串。
- Chat 合成成片与缓存成片均为 116,975 bytes，SHA 同为 `f446686f61619672b39c36a70099143130788d2fdbc71c19d2ac0bacb32ad7de`，证明命中同一缓存产物。
- 后处理 insert/delete/replace 分别为 10/9/11 秒，均保留 H.264 视频和 AAC 48kHz 双声道音轨。
- 早期删除 provider 视频曾留下 `workflow_run #54 running`。修复统一删除入口后新增 PostgreSQL 回归测试，并用真实 CLI 将隔离残留推进 `cancelled`；最终视频相关非终态 workflow/task 为 `0`。

### 漫画

- 漫画项目 `#7` 来源 snapshot `#2`；pipeline workflow `#59 completed`，提示词优化 workflow `#63 completed`。
- 数据库存在 11 条页/格 prompt 记录，已生成页绑定 `generated_image #19`。
- 页图为有效 RGB PNG，640x360、26,458 bytes，SHA-256：`4a55b5f936fd0481bd9dcb1b355e441ee181b5f1c2104fe1d310b9bb02adaec1`。
- 用户文档称“漫画 8 步”，实现实际进度合同为 7 个 LLM 阶段；拍摄剧本投影是确定性前置步骤，因此报告按“投影 + 7 个 LLM 阶段”记录。

### GALGame / WebGAL

- GALGame 项目 `#8` 来源 snapshot `#3`；pipeline workflow `#66 completed`，progress `7/7`。
- `galgame_script #1` 状态 `ready`；3 个场景、9 行剧本、2 个选择项。
- 两个选择项分别跳转 `scene_002_2.txt` 和 `scene_003_3.txt`；两条支线都用 `changeScene` 返回主场景。
- 主场景实际引用三张背景/CG、serious/surprised 人物差分、BGM 和 SFX；所有引用文件都存在。
- ZIP 15 个条目，190,966 bytes，`unzip -t` 无错误，SHA-256：`5ad96773a87e8e3ef00c8280c388599ad7dcf31514c1dce29534c58d84bb7d37`。
- 被引用 PNG 均为 640x360；BGM 为 PCM s16le 44.1kHz 双声道 2 秒；SFX 为 PCM s16le 48kHz 单声道 0.5 秒。
- ZIP 额外包含一个未引用的 1x1 透明 `figure_1_1.png` 占位图，不参与脚本引用，不影响工程闭合。

### 当前导出产物

| 产物 | 大小 | 结构检查 | SHA-256 |
|---|---:|---|---|
| TXT | 2,093 bytes | UTF-8，命中章节正文 | `d75fc371b9d9d77b1b8972d27059a9c50a3ff75dc6a3ce93e9b7b1064bd870dd` |
| Markdown | 2,135 bytes | UTF-8，书名和正文存在 | `46b219965ddaf7e8f680b6348977dcb1f1e827bf1af02ae7c5a0badb68a69533` |
| EPUB | 3,877 bytes | 合法 EPUB，`unzip -t` 无错误 | `fd86fdc85275e520bd2bcd4ceea0ac17738382b89861d91cb76b44db96a98978` |
| PDF | 4,185 bytes | `%PDF-`，PDF 1.4，2 页 | `57ee09f5f1547caf35e6c03d9a459f7dc3fe385f78a20bdc6ac3f00016eebe58` |
| 项目族 ZIP | 15,214,191 bytes | manifest、派生项目和媒体齐全，`unzip -t` 无错误 | `3f14c48a99f2fa00b4c108207a0daf18cfbcd46a6b79e63ca86b9f0873311a20` |
| GALGame 派生包 | 136,070 bytes | manifest 与媒体齐全，`unzip -t` 无错误 | `728d5b2d51e0ccf744d93068a0007c643d3eed3d65aeabfef28bfaa66f4b60ad` |

## 发现并修复的共享根因

1. 人物立绘 inline 路径绕过正式 generation stream。现统一经 generation task 启动、执行、终态校验和 `result_ref` 回读。
2. 普通图片和人物立绘把“请求尺寸”当成“真实媒体尺寸”写库。现由 `image_dimensions_from_bytes` 统一读取实际字节尺寸。
3. generation completion 未从全部 text stream snapshot 生成 `final_text_hashes`。现统一在 `complete_with_result` 收敛。
4. GALGame pipeline 预览未装配结构化拍摄剧本。现把装配规则收敛到共享 read model，预览和正式运行使用同一来源。
5. WebGAL 导出拒绝 `{"transition":null}`，但该值与缺省 transition 语义相同。现统一视为未设置并有定向测试。
6. 视频 Provider 原始响应中的 Base64 媒体可能再次进入 `metadata_json`。现 Host 持久化入口只保存 artifact 摘要。
7. 视频 insert/delete/replace 后处理固定 `-an`，会丢失音轨。现按输入探测音轨、补静音并用交错 video/audio concat 保留声音。
8. 视频提示词/采用/参考媒体身份曾存在旧字段旁路。现使用当前共享 segment plan、recipe 和 execution identity 校验，避免跨规划误采用。
9. 删除仍在 Provider 轮询的视频不会终止 `video_provider_job` workflow。现统一删除入口先取消关联 workflow，再清理媒体和记录。
10. 完成态音频资产没有从真实媒体读取时长，GALGame BGM/SFX 和画布音频可能写入 `duration_secs=NULL`。现统一使用 FFprobe 校验音频流并读取正时长，探测失败时回滚文件，媒体资产入口拒绝再次写入完成态空时长音频。
11. 源码模式 `a4n local --full` 和首次 managed-sidecar 构建只在启动 Server 时配置 ONNX Runtime，Cargo 链接阶段拿不到 `ORT_LIB_PATH`。现由同一 helper 同时配置 Cargo 构建命令和 Server 运行命令；在显式清除外部 `ORT_LIB_PATH` 后，full profile 已真实构建并完成生命周期验收。
12. CLI 媒体下载曾把 access token 放入 URL query，存在被代理或访问日志记录的风险。现 CLI 使用 Authorization Bearer header，Server 保留浏览器媒体标签所需的 query 兼容；subprocess E2E 同时断言 URL 不含 token、header 存在且失败覆盖不破坏旧文件。

主要修改位于：

- `crates/a4n-runtime/src/actions/portrait_generation/`
- `crates/a4n-runtime/src/actions/video_generation/provider_host.rs`
- `crates/a4n-runtime/src/actions/video_records/assembly/`
- `crates/a4n-orchestrator/src/context/generation.rs`
- `crates/a4n-orchestrator/src/media.rs`
- `crates/a4n-orchestrator/src/media/image_generation/`
- `crates/a4n-orchestrator/src/galgame/`
- `crates/a4n-orchestrator/src/video/records.rs`
- `crates/a4n-media/src/postprocess.rs`
- `crates/a4n-cli/src/client.rs`
- `crates/a4n-cli/src/local.rs`
- `crates/a4n-server/src/routes/media.rs`
- `scripts/tests/mock_openai_provider.py`

## 回归验证

| 验证 | 结果 |
|---|---|
| `a4n-cli` unit | 25 passed |
| `a4n-cli` subprocess E2E | 5 passed |
| `a4n-api-contract` | 12 passed |
| `a4n-service-protocol` | 69 passed |
| `a4n-service-manager` | 36 passed |
| `a4n-media --lib` | 15 passed |
| `a4n-media` FFmpeg/FFprobe smoke | 2 passed；实际输出含音轨，真实 WAV 时长探测通过 |
| `a4n-orchestrator` GALGame 相关 | 157 passed |
| `a4n-runtime` GALGame 相关 | 15 passed |
| 删除视频终止 Provider workflow | 1 passed（真实 PostgreSQL） |
| 图片实际尺寸、生成 trace、completion hashes、WebGAL transition、视频 metadata | 定向测试均 passed |
| `cargo build -p a4n-server` | passed |
| `cargo check -p a4n-tauri --no-default-features` | passed |
| `cd frontend && npm run build` | passed，2364 modules transformed |
| `npm run check:web-api-parity` | passed；前端调用缺失 Web/桌面/CLI 映射均为 0 |
| `npm run test:run` | 18 files、88 tests passed |
| `a4n local` core-only | 真实 PostgreSQL migration、health、381 命令 compatibility、owned Server graceful stop 均 passed |
| `a4n local` managed sidecar | 8 个隔离 sidecar + 外部 Qdrant 启停和端口回收均 passed |
| `a4n local` owned Qdrant | 隔离 HTTP/gRPC 端口、owned PID 和 graceful stop 均 passed |
| `a4n local --full --no-qdrant` | 在外部 `ORT_LIB_PATH` 未设置时完成源码链接、embedded Server 启停、health、381 命令 compatibility；passed |
| CLI 媒体 Bearer 下载 | 真实 PostgreSQL/Server 登录后下载字节一致；请求 URL 无 token、Server 日志无 token、owned Server graceful stop；passed |

## 已知边界

- GitHub/Google OAuth 需要真实客户端凭据和外部回调环境；本轮只验证未配置门禁，未调用第三方服务。
- 浏览器本地草稿恢复、复制到系统剪贴板、页面导航和播放器 UI 不属于无前端 CLI；服务端正文保存、导出和媒体下载已覆盖。
- 验收时创建的 BGM/SFX 两条历史测试资产数据库记录仍为 `duration_secs=NULL`；代码现已保证后续 GALGame/画布音频导入写入真实时长。旧记录不会静默猜测或自动改写，需要重新导入，或后续通过显式修复命令补录。
- 测试 Provider 验证的是 A4N 的真实业务链和 OpenAI-compatible 合同，不代表每个商业 Provider 的在线额度、网络和供应商可用性。
- Windows PowerShell release smoke 只能由 Windows CI 权威验证；本机只完成脚本人工审计和跨平台 release manifest 结构校验。

## 隔离资源收尾

- 本轮自定义端口 `18080`、`18641-18648`、`18765` 均已无监听，隔离临时目录 `/tmp/a4n-cli-flow.x0Oz4k` 已不存在；两者是在最终检查前被外部流程停止或移除，并非本次收尾操作主动终止。
- Qdrant 服务保持停止状态；确认 aliases 和 raft 元数据均未引用隔离 collection 后，已将唯一残留目录 `a4n_cli_flow_20260730_043610__b7d55bf47057511385c8141871bf7635` 移入系统回收站。原 storage 中已无该前缀，可从回收站恢复。
- 共享 `a4n-postgres` 容器在收尾时已处于停止状态。为避免改变共享服务状态，本次未重启容器，因此无法在线权威确认两个隔离数据库是否已被外部流程删除；该限制不影响验收期间已经在线确认的 workflow/task 终态和业务证据。
