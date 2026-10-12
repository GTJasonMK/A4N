# 性能审查与优化台账

本台账随基准和源码提交，后续优化先核对 ID 与已有测量，避免重复处理。

日期：2026-09-27—2026-09-28。基线：`88ee18431`，工作树：`work/debug`。用户授权直接修改 `debug`；开始时的 8 个前端页面 WIP 保持原样。原始 diff 和测量输出保存在 `data/performance-audit-20260927/`。

## 验收口径

- 每项修改必须先确认调用链、输入规模和重复频率；以同输入、同构建参数的前后对照保留证据。
- 本次采用目标场景耗时减少至少 30%，或进程峰值 RSS 减少至少 20% 的保留标准。短输入不达该比例不隐瞒；明显退化必须复查。
- 局部 CPU、RSS、资源体积与整条 HTTP 请求的耗时分别记录，不把算法基准写成全项目响应时间提升。
- 保持 API、权限、错误、取消、输出内容与排序语义。没有新增业务缓存、预取、后台任务或依赖。
- 测量与正确性检查分开。Rust 测量用优化构建，前端用生产构建和真实 Chromium；不把开发服务器启动当作验收。

## 已完成的优化

| ID | 请求链路、规模与频率 | 根因与最小修复 | 验证状态 |
| --- | --- | --- | --- |
| [PERF-001](../../crates/llm/a4n-llm-http/src/sse.rs) | 模型 HTTP → OpenAI SSE → 生成事件；每个网络数据块；覆盖 1/32/256/2048 个事件 | 逐行 `drain().collect()` 重复搬移尾部并分配临时行；改为借用行切片，批次结束只整理一次尾部 | 2048 事件批次 **6.907 → 1.252 ms，下降 81.87%** |
| [PERF-002](../../crates/llm/a4n-llm-http/src/anthropic.rs) | Anthropic SSE → 生成事件；同上 | 同一逐行搬移问题；取出原缓冲区解析后归还，保持容量与解析顺序 | 2048 事件批次 **7.865 → 1.221 ms，下降 84.48%** |
| [PERF-003](../../crates/rag/a4n-rag-core/src/semantic_chunker/matrix.rs) | 章节资料同步 → 服务端语义分块 → 相似度 → 前缀和 → 动态规划；每次正文索引，1600 句 × 768 维 | 同时保留两个平方级矩阵；相似度矩阵预留零边界，原地转换前缀和。保持原来的浮点运算顺序，不改变评分算法 | 峰值 RSS **48,552 → 28,256 KiB，下降 41.80%** |
| [PERF-004](../../frontend/src/hooks/usePacedStreamText.ts) | 写作台／灵感对话／Agent 消息／进度弹窗；1千/1万/5万字符，每秒最多 120 次显示更新 | `useRef` 的初始化参数被每次渲染重新计算，造成无用的整段 `Array.from`；对已有字符 ref 做惰性初始化，保留原有字符推进和自适应速率算法 | 1万字符、1000 次更新 **117.7 → 17.6 ms（85.05%）**；5万字符 **527.8 → 15.1 ms（97.14%）** |
| [PERF-005](../../frontend/src/i18n/format.ts) | 全站列表、进度、图表 → 国际化格式化；每次渲染每个数字／相对时间 | 每次调用都构造 `Intl` 格式器；复用两种固定语言的不可变格式器，自定义数字选项仍独立处理 | 两种语言、两种格式的 2000 次调用分别下降 **95.82%–96.96%** |
| [PERF-006](../../crates/platform/a4n-db/src/workflow/runs/projections.rs) | 工作流列表／详情 → 公开进度投影；含 0/100/1000 个已完成单元的恢复快照 | 复制完整 `progress_json` 后立即丢弃恢复快照；现在只复制公开字段，并保留快照类型与进度字段校验 | 1,103,968 字节输入 **244.812 → 0.689 µs，下降 99.72%**；这是投影函数耗时 |
| [PERF-007](../../crates/platform/a4n-db/src/project/progress/shared.rs) | 项目列表／摘要 → 漫画、游戏、自由视频的流水线进度；每次读取，12 项目、每条快照 540 KB | 只显示消息却从数据库取回整个恢复快照；SQL 直接提取 JSON 消息值，保留最新记录选择、状态和错误优先级 | 真实 PostgreSQL 查询及投影 **20.237 → 6.309 ms，下降 68.82%** |
| [PERF-008](../../frontend/src/components/ui/MarkdownText.tsx) | Agent／灵感对话 → 历史 Markdown；新回复每次更新，50 条历史、40 次更新 | 未变化的历史文本反复解析、构建和协调 DOM；使用 React 原生 `memo` 隔离这个仅依赖两个字符串属性的纯组件 | **265.6 → 1.9 ms，下降 99.28%**；最终 DOM SHA-256 完全相同 |
| [PERF-009](../../crates/host/a4n-edge/src/lib.rs) | 浏览器冷启动 → Edge → 生产 JS/CSS/HTML；每次空浏览器缓存访问 | Edge 原先发送完整文本；[Vite](../../frontend/vite.config.ts) 在构建时生成 gzip，Edge 原生协商预压缩表示并设置 `Vary: Accept-Encoding`；Tauri 构建跳过额外表示 | 4 Mbps／50 ms／4 倍 CPU 条件下，登录表单可操作时间：1440px **2772.6 → 950.8 ms（65.71%）**，390px **2717.7 → 956.3 ms（64.81%）** |
| [PERF-010](../../frontend/src/api/generationEvents.ts) | 生成页面加载 → generationTasks → 协议校验器；每个浏览器文档首次使用 | 固定 schema 每次在浏览器编译；[合同生成器](../contracts/generate-generation-validators.mjs) 直接生成 ESM 校验器，保留相同 AJV 选项、Unicode helper、所有字段校验和错误 | 4 倍 CPU、冷模块导入并校验首批 3 个真实协议样本：**366.7 → 11.1 ms，下降 96.97%**；2387 组决策及完整错误对象对照一致 |
| [PERF-011](../../crates/platform/a4n-db/src/workflow/runs/queries.rs) | 章节列表／单章摘要 → Chapter Worker／Orchestrator → 工作流与检查点；100 章、每章 3 次历史、每次两个 64 KB 检查点 | 只使用最新运行却读取全部历史并深复制检查点；数据库按原排序直接选择所需章节的最新运行，检查点移交所有权 | 8 并发完整 HTTP 响应 P95 **174.54 → 82.08 ms（52.98%）**；整栈读取后 PSS **1,236,524 → 637,804 KiB（48.42%）** |
| [PERF-012](../../crates/llm/a4n-llm-http/src/errors.rs) | 模型网关／内嵌 provider → HTTP/TLS → SSE；同一超时配置下连续 45 次请求，每次更换模型和测试凭证 | 每次构造 provider 都重建 HTTP Client、证书库和连接；复用原生 Client，保留独立的连接／逐次读取超时，最多保留 16 种配置 | 本机 TLS/SSE 请求中位数 **10.770 → 0.603 ms（94.40%）**，P95 **14.658 → 0.750 ms（94.88%）**；45 次请求由 45 条连接降至 1 条 |
| [PERF-013](../../frontend/src/pages/writing-desk/ChapterSidebar.tsx) | 写作桌每次输入／流式正文更新 → 侧栏；100／1000 章，每章 4000 个 Unicode 码点，25 次更新 | 所有已保存正文重复计数且全部章节行重复渲染；字数随现有章节索引计算，当前章实时计数；[章节行](../../frontend/src/pages/writing-desk/ChapterCard.tsx) 通过原有稳定切章回调和 React `memo` 复用未变化的渲染 | 100 章 **182.4 → 10.1 ms（94.46%）**；1000 章 **1646.5 → 39.7 ms（97.59%）**；这是侧栏 25 次更新的累计 CPU 时间，完整 DOM 一致 |
| [PERF-014](../../crates/embedding/a4n-embedding-fastembed/src/provider.rs) | Embedding Service／动态 provider → FastEmbed → ONNX；已下载的默认中文模型，1／32 条文本、1／4 并发 | 每次 RPC 的 provider 释放后丢弃已初始化的模型；共享同模型／目录的模型资源，额外只保留一种空闲配置，阻塞推理持有模型锁至真正结束 | 单条查询中位数 **428.07 → 34.72 ms（91.89%）**；32 条、4 并发 **2340.93 → 1268.56 ms（45.81%）**；峰值 RSS **599,124 → 229,788 KiB（61.65%）** |
| [PERF-015](../../crates/embedding/a4n-embedding-providers/src/http.rs) | API 嵌入 → HTTPS `/embeddings`；1／32 条文本、384 维、连续 45 次请求 | 固定 60 秒总超时的 HTTP Client 随每次 provider 重建；原生单例 Client 保留连接池，模型／凭证／维度／批量参数仍逐次构造 | 单条中位数 **20.830 → 1.468 ms（92.95%）**；32 条 **30.788 → 13.299 ms（56.80%）**；45 条连接降为 1 条，完整响应一致 |
| [PERF-016](../../crates/host/a4n-edge/src/lib.rs) | Web → Edge → 大型 JSON；真实模板语料组成约 1.1／11 MB 响应，4 Mbps／50 ms | JSON 原先按完整字节交付；启用现有 Tower HTTP 的 gzip，最快压缩等级，只处理符合条件的大型 200 JSON 响应 | 三轮中位数 **2332.8 → 1073.3 ms（53.99%）／22225.3 → 9826.0 ms（55.79%）**；全部解压后字节摘要一致 |
| [PERF-017](../../frontend/src/pages/WritingDesk.tsx) | 写作桌/只读目录/导入预览 → Chapter Content Worker；1000章、4Mbps/50ms | 目录原先携带整本正文/分析；目录窄投影与带项目归属的单章读取共用权限/状态规则，正文按选择读取 | 页面可编辑中位数 **5264.6 → 741.1 ms（85.92%）**；真实千章目录 **11,601,686 → 742,686 字节（93.60%）**；业务/权限/草稿回归通过 |
| [PERF-018](../../frontend/src/pages/novel-detail/story-events/ChapterEventSection.tsx) | 故事事件筛选 → 章节卡片渲染；100／1000 章，每章 5 事件，连续 8 次含收窄和放宽的筛选 | 原过滤为所有匹配章节创建新对象，未变化卡片也反复渲染；完整保留的章节维持原对象，卡片使用 React 原生 memo | CPU18/19 三轮交错中位数 **135.2 → 50.4 ms（62.72%）／1328.1 → 503.9 ms（62.06%）**；完整 DOM 和筛选行为一致 |
| [PERF-019](../../crates/platform/a4n-db/src/chapter_content/queries.rs) | 故事事件初载/刷新 → Control → Chapter Content Worker → PostgreSQL；1000章、900章有分析、4500事件 | 旧接口读取正文并生成未使用的同步状态；新分析读口仅选取 id/章号/标题/摘要/分析五列，复用项目权限和删除过滤 | 三轮真实 API P50 **126.56 → 43.34 ms（65.76%）**，P95 **160.61 → 66.36 ms（58.68%）**；JSON **13,650,788 → 2,172,288 字节（84.09%）** |
| [PERF-037](../../crates/platform/a4n-db/src/project/bundle.rs) | 用户触发的独立漫画项目包导出；128 个来源章节、32 个文本资产，每资产 2 个版本 | 原先逐资产验证并读版本、逐来源并发读章节/快照/快照章节；改为 1,000 ID 分批查询、按 ID 映射并保留来源排序与原错误 | 真实 PostgreSQL 完整导出 SQL **484 → 10**；P50 **51.346 → 5.365 ms（89.6%）**，P95 **51.885 → 6.007 ms（88.4%）**；完整序列化包逐项相同 |
| [PERF-038](../../crates/platform/a4n-db/src/project/bundle.rs) | 用户触发项目包导入中的对话历史阶段；1 个会话、1,000 条消息，每条约 280 B | 原实现每条消息执行单独 INSERT；同一事务中将消息按 1,000 条分批后执行 `insert_many`，导入计数和会话映射规则不变 | 真实 PostgreSQL 历史导入 SQL **1003 → 4**；同时间戳排序复测 P50 **86.847 → 14.987 ms（82.7%）**，P95 **98.599 → 18.369 ms（81.4%）**；消息字段和顺序一致 |
| [PERF-039](../../crates/platform/a4n-db/src/project/bundle.rs) | 用户触发项目包历史导入中的 workflow progress；单 workflow 保留上限 200 条事件 | 原实现每条事件执行单独 INSERT；先映射 workflow ID，再按 1,000 行批量插入，task_id 清理与 JSON 重映射不变 | 真实 PostgreSQL SQL **203 → 4**；P50 **29.598 → 6.707 ms（77.3%）**，P95 **65.253 → 13.312 ms（79.6%）**；事件字段、时间与 event_index 顺序一致 |
| [PERF-040](../../crates/platform/a4n-db/src/project/bundle.rs) | 用户触发项目包历史导入中的模型用量账本；1,000 条记录关联单个 workflow | 每条记录重新执行 request ID 生成、user/project/workflow 映射与 pricing 脱敏后单独 INSERT；保留转换规则，在同一事务内每批 1,000 条执行 `insert_many` | 真实 PostgreSQL SQL **1003 → 4**；P50 **192.535 → 42.019 ms（78.2%）**，P95 **305.936 → 52.018 ms（83.0%）**；映射、账本字段与 JSON 内容一致 |
| [PERF-041](../../crates/platform/a4n-db/src/project/bundle.rs) | 用户触发项目包导入中的模型调用追踪；1 个 workflow、1,000 条 LLM trace | 原实现逐条映射 project/root/workflow/generation ID、脱敏上下文 JSON 并 INSERT；按 1,000 行分批写入，字段转换不变 | 真实 PostgreSQL SQL **1003 → 4**；P50 **314.611 → 50.811 ms（83.8%）**，P95 **526.321 → 74.528 ms（85.8%）**；trace 字段逐项一致 |
| [PERF-042](../../crates/platform/a4n-db/src/project/bundle.rs) | 用户触发项目包导入中的 generation target 关联；1 个 generation run、1,000 个 target | 原实现逐条映射 generation run / domain entity 并 INSERT；映射转换不变，按 1,000 行批量插入 | 真实 PostgreSQL SQL **1004 → 5**；P50 **88.124 → 14.407 ms（83.7%）**，P95 **115.431 → 22.759 ms（80.3%）**；target 表/ID、kind、role、时间与序列顺序一致 |
| [PERF-043](../../crates/platform/a4n-db/src/asset/generation.rs) | 章节分析物体发现持久化；单次 16/32/64/128 个档案 | 首次事实沿用已写入的当前投影，档案结果由逐条回读改为按 ID 批量读取；事实审计、锚点证据和事务语义不变 | PostgreSQL P50 **55.350/36.873/45.468/91.485 → 9.991/16.905/29.935/41.031 ms**，分别下降 **82.0%/54.1%/34.2%/55.1%**；P95 降幅 **84.3%/65.4%/78.0%/51.3%** |
| [PERF-044](../../crates/platform/a4n-db/src/agent/thread_commands.rs) | Agent thread history 请求；32 个 Run，每次打开历史读取消息、运行、执行和预算投影 | 旧实现对每个 Run 单独计算预算快照；复用已有 `run_budget_snapshots` 一次读取所有 Run 家族和执行事实，按 Run 回填 usage | PostgreSQL P50/P95 **8.185/11.335 → 2.766/3.851 ms**，下降 **66.2%/66.0%**；预算查询约 **96 → 3** |
| [PERF-045](../../frontend/src/pages/novel-detail/StoryEventsTab.tsx) | 故事事件页初载；1000 章、5000 事件，伏笔索引 1000 条且默认收起 | 初载不再读取默认收起的伏笔索引；用户展开时按需读取，刷新和展开共享同一项目的 in-flight 请求，保留完整列表、错误和键盘语义 | 真实 Chromium 153 受控夹具三轮中位数 **342.7 → 196.2 ms（42.7%）**；初始伏笔 JSON **505,677 → 0 B**；展开后 1000 条记录完整渲染 |
| [PERF-046](../../frontend/src/components/agent/AgentMessageList.tsx) | Agent Monitor／领域 Agent 控制台高频状态更新；1000 条消息、80 次与消息无关的重渲染 | 消息列表组件按消息数组、流式 ID、待发送消息和运行状态做语义比较；无关输入/活动状态更新跳过整段分组与行协调，消息变化仍完整渲染 | 真实 Chromium 153 受控夹具三轮中位数 **149.6 → 0.1 ms（99.9%）**；1000 行 DOM 与首尾 HTML SHA-256 一致 |
| [PERF-070](../../frontend/src/pages/AgentMonitor.tsx) | Agent Monitor／领域 Agent 控制台在 SSE 正常时每 7 秒完整读取一次 thread snapshot；快照最多包含 1000 条消息及运行、执行、事件投影 | SSE 收到数据块（含 keep-alive）或桌面事件轮询成功后的 10 秒内停止完整快照轮询；流断开或静默后恢复 7 秒兜底，事件游标、缺口重载和生命周期重载不变 | 生产轮询调度单元基准：健康连接 28 秒窗口旧路径 **4 → 0 次完整快照（100%）**；断开仍 **4 次**，无数据超过 10 秒后恢复；未宣称单次快照响应时延下降 |
| [PERF-047](../../crates/platform/a4n-db/src/profile_material/queries.rs) | 对象/场景档案列表计数与删除下游图片 ID；20,000 张素材图片，单条元数据约 1.75 KB | 旧实现把该项目该类型所有 `metadata_json` 读回进程再逐条解析；计数改为 PostgreSQL 按 `profile_kind/profile_id` 聚合，删除改为数据库按 JSON identity 直接筛选 ID，数值类型和正整数校验仍在 Rust 侧保留 | 计数 PostgreSQL P50/P95 **277.905/307.716 → 39.846/44.813 ms（85.7%/85.4%）**；删除 ID P50/P95 **275.986/283.187 → 27.412/36.981 ms（90.1%/86.9%）**；计数和 ID 顺序逐项一致 |
| [PERF-048](../../frontend/src/pages/derived/useMediaAuthoring.ts) | 漫画/Galgame 媒体阶段标签；1000 章、1000 份同类文档、80 次状态更新 | 旧实现每个章节范围反复过滤全部文档并再次扫描缺失版本；一次建立当前快照的 draft/adopted 索引与历史存在集合，状态优先级和 stale 语义不变 | 真实 Chromium 153 受控三轮中位数 **100.3 → 6.2 ms（93.8%）**；1000 章/1000 文档和标签文本一致 |
| [PERF-049](../../crates/platform/a4n-db/src/project/dependency_graph/graph/records/family.rs) | 依赖图初载 → 族级图片摘要投影；12,000 张图片，其中 600 张参与图计数，项目打开时每次读取 | 旧实现把项目族所有 `generated_image.metadata_json` 反序列化后再按类型计数；图摘要查询增加实际消费的 6 个图片类型过滤，节点详情仍走完整独立读取 | PostgreSQL 图摘要投影 P50/P95 **56.783/65.582 → 3.324/5.278 ms（94.1%/92.0%）**；返回行 **12,000 → 600**，元数据字节 **42,942,240 → 32,640**，图计数逐项一致 |
| [PERF-050](../../crates/platform/a4n-db/src/workflow/tasks/queries.rs) | `get_task_history` → task worker → PostgreSQL；200 条任务历史页，单条持久 payload 约 32.5 KB | 旧查询读取完整 `task_queue` 行，包括 API 摘要不使用的 `payload` 和 `dag_id`；历史及按 ID 读取改为显式选取状态投影字段，共用原校验和错误映射 | PostgreSQL P50/P95 **7.266/13.174 → 2.096/2.677 ms（71.1%/79.7%）**；旧路径该页 payload **6,505,200 B**，新路径不读取 payload；字段、顺序、失败错误和进度投影一致 |
| [PERF-051](../../crates/platform/a4n-db/src/workflow/runs/queries.rs) | Canvas workflow 状态每秒轮询及设置页展开详情；最多 300 个 generation run | UI 只使用 generation run 状态、标题、次数和 JSON 存在标记；UI 显式请求省略 fingerprint/raw/parsed/validation/artifact JSON，CLI 缺省仍取完整诊断内容 | 完整 workflow detail **17,324,230 → 195,978 B**；真实 PostgreSQL P50/P95 **18.561/26.785 → 2.528/3.145 ms（86.4%/88.3%）**；300 个项目顺序、摘要字段和 3 个内容存在标记一致 |
| [PERF-052](../../crates/host/a4n-runtime/src/actions/admin_records/test_console.rs) | 测试中心活动 run 详情每 800 ms 轮询；最多 500 条日志，每条 append 日志最多 2,000 字符 | 默认/旧请求仍返回完整日志；前端轮询带独占 log cursor，后端只克隆和序列化 cursor 之后的日志，并从缓存详情中投影摘要/步骤 | release TestConsole read + JSON serialization：500×2,000 字符时响应 **1,001,914 → 415 B**；P50/P95 **0.676/0.947 → 0.001/0.002 ms**（P50 下降 99.8%）；日志轮转和过期游标语义一致 |
| [PERF-053](../../crates/platform/a4n-task-runtime/src/channel/handle.rs) | `useProgressTask` 每 500 ms 请求和 Canvas workflow 每秒请求，最多 200 个历史事件；活体文本尾部固定 512 字符 | 两处 UI 将最近事件 ID 作为可选 cursor；task 内存 ring 从游标后克隆事件，持久 workflow 查询按 ID 过滤；无游标的旧调用仍返回最近完整页，stream text 仍每次返回 | 200 条×256 字符时 task response **71,031 → 894 B**、P50/P95 **0.332/0.420 → 0.004/0.006 ms（98.7%）**；隔离 PostgreSQL workflow response **73,213 → 398 B**、P50/P95 **3.068/5.256 → 0.639/1.016 ms（79.2%/80.7%）**；cursor 后只保留新增事件 |
| [PERF-054](../../crates/host/a4n-runtime/src/actions/admin_records/test_console.rs) | Test Center `list_test_runs(30)` 扫描全部 JSON 报告；每份最多 500×2,000 字符日志 | 终态/缓存报告走 typed summary projection；仅 stale queued/running 报告完整解析并恢复，错误类型和列表排序保持一致 | 40 份报告的 response bytes 相同，P50/P95 **17.778/37.523 → 11.510/14.530 ms**（35.3%/61.3%），peak RSS **51,268 → 13,004 KiB**（74.6%） |
| [PERF-055](../../crates/platform/a4n-db/src/project/chapter/context/generation.rs) | 每次章节生成构造人物参考上下文；16/32/64 个有首见证据的人物档案 | 首见证据、人物时间线/归属、反馈事实改为 owner 内批量查询；时间线每 128 档案分块，保留旧投影与排序语义 | PostgreSQL 交错 A/B 的 P50/P95：16 档 **62.964/75.181 → 7.428/9.614 ms**；32 档 **127.599/141.437 → 8.199/10.110 ms**；64 档 **247.041/290.149 → 7.523/10.703 ms**；档案响应字节逐项相同 |

较小输入同样保留记录：OpenAI 1/32/256 事件分别下降 6.66%/6.74%/26.08%，Anthropic 分别下降 7.45%/10.30%/35.55%。1000 字符、995 次显示更新为 27.4 → 20.1 ms（26.64%）。这些没有达到 30% 的小输入结果不作为主要验收收益，不把压力批次的提升外推到单事件请求。

Intl 的四组对照（每组 2000 次）：中文数字 75.7 → 2.3 ms、中文相对时间 69.3 → 2.9 ms、英文数字 74.8 → 2.3 ms、英文相对时间 78.3 → 2.7 ms。自定义格式选项和日期时区路径没有改变。

## 审查覆盖与未修改项

| 范围 | 已核对事实与决定 |
| --- | --- |
| 首屏和路由 | `App.tsx` 已按路由和依赖图弹窗懒加载。PERF-009 已通过真实 Edge 和生产登录页验证压缩交付；同一份入口文件从 1,010,285 原始字节变为 310,789 传输字节。两种语言字典仍静态加载；临时 locale 拆分上界只减少 28.8% gzip 和 13.9% GC 后 heap，未达到门槛，因此未改实现。 |
| 项目列表 | `ProjectReadWorker::summary_for_projects` 已批量查询项目族、权限、进度和反馈间隔；不重新引入逐项目请求或重复批处理层。 |
| 章节列表 | PERF-011 消除未使用历史读取及检查点深复制，PERF-017 将写作桌/只读目录/导入预览改为按需读取。原 `ChapterSummary` 完整合同保留，目录与正文类型分开；故事事件由 PERF-019、来源快照由 PERF-020 单独覆盖。 |
| 数据库权限和协作 | 读取使用连接池、绑定参数和批量根项目查询。PERF-059 用 20 项目 × 1000 成员测量了项目列表成员聚合候选，完整访问摘要 P50/P95 只下降 8.8%/24.8%，已拒绝；线上团队规模分布仍未证明更大的协作组是常见负载。 |
| 工作流轮询和恢复 | 旧任务进度轮询间隔 500 ms，已有单次进行中保护和结束清理；新生成任务走事件订阅。未通过减少进度频率来换取表面收益。恢复快照的读取需求与公开进度投影分开评估。 |
| 事实反馈轮询 | PERF-056 限制摘要只读最新 100 条变更；PERF-069 在已应用反馈覆盖稀疏时，先按章节 ID 过滤来源修订，跳过无匹配反馈记录的正文/分析哈希。1.5 秒轮询和完整覆盖路径保持原校验。 |
| Agent 维护扫描 | PERF-029 批量读取外部等待与预算；PERF-030/031 批量读取仍活动的 Generation/Workflow owner 预算；PERF-034/036 测量 root/child 过期 lease 的完整恢复批次；PERF-035 为高基数 Generation owner JSON 关联增加表达式索引。审批和编辑器等待的局部预读候选未达门槛并拒绝保留。 |
| HTTP 与媒体 | Worker／service client 已持有并复用 HTTP client；Web 媒体使用 `ServeFile`，Edge 使用流式 body。桌面自定义协议要求字节响应，不能直接套用 Web 的传输方案。 |
| LLM 上游连接 | PERF-012 保留每次构造 provider 的配置与凭证隔离，仅延长 HTTP Client 的生命周期。原连接／逐次读取超时、错误、取消与背压检查通过；不缓存模型响应。 |
| 嵌入模型生命周期 | PERF-014／015 已处理本地模型和 API 连接的重复初始化。模型文件删除／下载会使额外持有的资源失效；仍存活的 provider 保留原有拥有权，避免破坏已在使用的模型。没有缓存业务向量结果。 |
| 语义分块 | 已复用向量范数并利用相似矩阵对称性；本次只消除额外矩阵。将全矩阵换成局部带状算法可能改变浮点抵消和相同分数时的切分，未作为无风险替换。 |
| 故事事件 | PERF-018 减少筛选时未变化卡片的重复渲染；PERF-019 从数据库到前端仅读取所需的五个分析字段；PERF-045 延迟默认收起的伏笔索引；PERF-058 将只读事件页的 GC 后 JS heap 降低 32.9%。千章/4500事件仍需渲染大量卡片，DOM 压缩和搜索候选均因未达门槛拒绝。 |
| Agent 消息流 | PERF-029/044 已覆盖 Agent 后端维护扫描和历史预算投影；PERF-046 处理无关父状态更新，PERF-065 让消息数组变化时只重渲染被替换的行及其配对工具结果。 |
| Video Studio Canvas | PERF-066 将运行结果失效和装配连线批量重排的重复全边扫描改为单次索引；PERF-067 将每秒耗时更新与仅按节点状态变化的活动范围/边投影分离。 |
| 档案素材与图片 | PERF-047 只优化档案列表图片计数和删除下游 ID 的读取；完整素材列表、主素材绑定和图片详情仍保留原字段与顺序。 |
| 依赖图 | PERF-049 将族级图摘要限制为漫画页、对象/场景素材和三类视频帧；节点详情仍按节点类型独立读取完整记录，未改变详情合同。 |
| 任务历史 | PERF-050 的历史页和按 ID 查询只读取摘要字段；完整任务 payload 仍留在任务详情/执行路径所需处，没有改变队列持久化或取消行为。 |
| 工作流详情轮询 | PERF-051 让 Canvas 每秒刷新及设置页详情请求只传输 generation run 摘要；未传新选项的 CLI 保留完整原始输出、解析结果和验证 JSON。 |
| 测试中心详情轮询 | PERF-052 首次读取与旧请求仍拿到完整保留日志；活动轮询带日志游标，只克隆和传输新增行；超过 500 行后的过期游标重取当前窗口。 |
| 任务进度轮询 | PERF-053 task ring 和 workflow progress query 支持 event cursor；缺省游标仍返回最近页，丢失 ring 历史后退回最近完整窗口，活体 stream text 继续按原快照返回。 |
| 大纲生成进度轮询 | PERF-068 将分部重叠验证从枚举整个章节区间改为按分部区间处理，完成统计按实际完成章节合并；3 秒轮询频率和权限校验保持不变。 |
| Canvas 文本素材 | 窄索引候选已用 256 份 × 4 KiB 正文的 Chromium 夹具复核；传输字节下降 98%，但首屏中位数仅下降 14.0%，GC 后 JS heap 仅下降 8.2%，不满足保留门槛，未改合同或生产代码。 |
| 媒体阶段状态 | PERF-048 将状态标签计算从章节数 × 文档数的重复过滤降为一次索引；文档状态、快照范围和 stale 判断保持原语义。 |
| 漫画来源读取 | 当前 MangaTab 产品入口 ComicProjectWorkspace、VideoProjectWorkspace 和 GalgameProjectWorkspace 均使用 `listSourceSnapshotChapterIndex`；PERF-020 已用 1000 章真实 PostgreSQL/Chromium 夹具测量目录与单章详情，完整正文只在选章后读取。旧 `listSourceSnapshotChapters` API 仍保留给兼容调用方，但不是当前页面活跃路径。 |
| Manga 活动状态渲染 | PERF-071 将每 2.5 秒更新的选中章节状态限制在实际变化的行；1000 章列表的渲染提交 P50/P95 分别下降 84.2%/75.6%，选择和最终 DOM 保持一致。 |
| Page Session lease 清理 | PERF-072 把 session 请求的全表过期扫描改成目标 session 检查，并将多个 SSE 连接的周期扫描合并为进程级 5 秒一次；1000 个 session/1000 次心跳批次的 registry 执行 P50/P95 下降 98.9%/98.9%。 |
| 项目包传输 | PERF-037 批量读取独立漫画包的文本资产版本与来源实体；PERF-038—042 批量写入历史消息、workflow progress event、模型用量账本、LLM trace 与 generation target。SQL 数随数据行数增长，未引入缓存或预取。 |

## 可复现入口

环境：Intel i9-13900H、Rust 1.95.0、Node 24.18.0、Chromium 150.0.7871.186、PostgreSQL 16。前端最终对照将测试进程及子进程固定到 CPU 18、19，前后交错运行三轮；没有停止其他开发会话。每轮预热至少 250 ms 且至少三次，再取九个样本中位数，最终取三轮中位数。Rust SSE 与 SQL 也进行三轮交错对照，RAG RSS 每次使用独立进程。

逐字显示测试在同一个 React root 上测量，并在采样前结算挂载／重置 effect；控制计时器推进，不把刻意配置的逐字等待时间算成 CPU 改善。中英文、emoji、替换、截断、排空、`wasStreamed` 和主题切换的完整行为轨迹前后相同。

前端（从 `frontend/` 执行；只服务本地测试夹具，不请求真实模型）：

```bash
env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
  NO_PROXY=localhost,127.0.0.1 node e2e/performance/check.mjs
```

设置 `A4N_PERF_BASELINE_REF=88ee18431` 可将被测的三个前端模块从原提交载入同一个测试夹具，避免切换分支或覆盖工作区；`A4N_PERF_ARTIFACTS` 指定独立结果目录。`A4N_PERF_REUSE_BUILD=1` 可复用已经生成的生产测试包进行交叉复测。用 `taskset -c 18,19` 启动上述命令可复用最终 CPU 亲和性设置；换机器时选择该机器实际可用的 CPU。行为对照覆盖 1440px／390px。

Rust（从当前工作树执行，持有 `work/.debug-build.lock`；首次先用 `--no-run` 完成编译，实际测试单次限制 60 秒）：

```bash
flock -x ../.debug-build.lock env CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_RELEASE_LTO=false timeout 60s cargo test --release --locked --offline \
  -p a4n-llm-http --lib performance_ -- --ignored --nocapture --test-threads=1
flock -x ../.debug-build.lock env CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_RELEASE_LTO=false timeout 60s cargo test --release --locked --offline \
  -p a4n-rag-core --lib performance_semantic_chunking -- --ignored --nocapture --test-threads=1
flock -x ../.debug-build.lock env CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_RELEASE_LTO=false timeout 60s cargo test --release --locked --offline \
  -p a4n-db --lib performance_ -- --ignored --nocapture --test-threads=1
```

最后一条包含数据库基准，必须显式提供隔离的 `A4N_TEST_DATABASE_URL`，不要指向日常开发库。本轮使用单独的 `a4n_perf_test_20260927`；每次基准再通过项目现有 migration test support 建立独立 schema。

Rust 的 RSS 使用独立测试二进制的 `wait4().ru_maxrss`；不把 Cargo 编译器内存计入运行时。RAG 每次输出全部分块字节长度，六次对照完全相同；额外用原提交的真实矩阵源码对比当前实现，45 组配置的所有前缀和与候选分块分数逐位相同。原始和优化后的 Rust 基准二进制保留在本地证据目录，未提交二进制。

最终证据目录为 `data/performance-audit-20260927/`：

- `summary.json`：全部量化结果、保留门槛与被测生产源文件 SHA-256。
- `frontend-comparison.json`、`frontend-{baseline-final,final}/report-round-*.json`：三轮浏览器样本、完整行为轨迹和 DOM 摘要。
- `rust-comparison.json`、`comparison-*-a4n_*.log`：SSE 耗时、RAG RSS 与分块结果。
- `database-comparison.json`、`comparison-*-pipeline-*.log`：实际 PostgreSQL 查询对照。
- `baseline-db-performance.log`、`progress-after-perf.log`、`matrix-equivalence.log`：投影微基准和矩阵逐位对照。

早期试验目录不属于最终验收数据。证据、测试数据库配置和临时类型检查快照均留在忽略目录中，不提交密钥、运行数据或构建产物。

## 最终验证

- 优化构建和定向回归：LLM HTTP 54 项、RAG 29 项、数据库项目进度 12 项、工作流 28 项，共 **123 项通过**。数据库用例使用真实 PostgreSQL，包含最新 ID 选择、项目与 workflow kind 范围、错误优先级、非字符串 JSON 和快照保留。
- 前端 Vitest **432 项通过**；生产 Chromium 对照通过；Host API、generation schema、UTF-8 fixtures 一致性检查通过。
- `cargo fmt --all -- --check`、`git diff --check` 通过；最新当前工作树的完整 `npm run build`（TypeScript + Vite）与 432 项 Vitest 重查均通过。
- 基线已有一条过时测试把空进度 `{}` 判为错误，但生产实现一直返回“没有公开进度”；已保留原行为并修正测试。未知字段、非法快照、缺字段、越界进度仍明确失败。
- 21:01 的整树 `npm run build` 曾被并行视频页面 WIP 的五处 `Promise<boolean>`／`Promise<void>` 不匹配阻塞，失败日志保留在 `frontend-build-final.log`。本任务没有改动那些视频文件；先验证了已提交基线 `3a05cb9f` 加本次三个前端文件的临时快照。并行改动完善后，21:25 的整树重查通过，见 `frontend-build-current.log`、`frontend-tests-current.log`。

## 第二批：真实首屏与校验器初始化

基于 `0c83a6728` 继续，原始证据位于 `data/performance-audit-20260927/b02/`。

- `startup-comparison/report.json`：旧／新 Edge 运行同一份生产资源，9 轮交错冷加载；后端仅提供固定的空 OAuth 提供者列表，不替换页面或静态文件服务。两种宽度的页面文本摘要一致，用户名／密码键盘流程通过，截图保留在同目录。
- 原入口实际传输 1,010,285 → 310,789 字节（69.24%）。构建生成 163 个压缩表示，共约 1.05 MB；没有运行时压缩 CPU 开销。`TAURI_ENV_PLATFORM=linux` 的前端构建中压缩副本数为零，沿用 Tauri 已有的构建环境标识。
- 实际 HTTP 检查覆盖原始内容与解压内容一致、MIME、HEAD 长度、`gzip;q=0`、identity Range、304、SPA 回退；API 响应不进入静态资源层。`Vary` 同时保留 CORS 的原有值。
- `schema-load/report.json`：原始源码与预编译实现使用同样的 Vite 库构建方式、同一个浏览器与同一批协议数据对照；计入模块下载、解析、初始化和首批事件校验。另在真实 Vite 开发服务器中导入该模块验证，开发／生产两种入口均通过。
- 校验器没有改变持续运行时的校验算法：3000 次热事件校验为 73.8 → 77.1 ms，不计作性能收益。预编译模块原始库构建体积 230,109 → 270,530 字节；gzip 后 52,722 → 18,652 字节。实际应用生成任务 chunk 的 gzip 大小约 49.5 KB → 18.8 KB。明确保留体积／首用成本的取舍，未通过删校验换速度。
- 曾试验关闭引用内联以减少生成代码，但完整错误对象的 `schemaPath` 发生变化，已拒绝该方案并恢复原选项；失败证据为 `rejected-inline-refs.log`。最终 2387 组包含嵌套字段缺失、类型、Unicode、未知字段和安全整数边界的对照完全一致。
- 合同生成的 `--write/--check` 同时写入／核对 `generationValidators.js` 与 `.d.ts`。使用 AJV 自身的 Unicode helper 生成 ESM，浏览器不再依赖运行时编译或 CommonJS 兼容补丁。
- 本批 Edge 5 项、前端 432 项、合同生成／fixture、完整前端构建、Tauri 前端构建、格式检查通过。中途他方 Canvas WIP 引起的类型失败保留在 `frontend-build-final.log`，后续当前工作树重查通过，见 `frontend-build-recheck.log`。

复现入口（在 `frontend/`，同前文清除测试进程的 HTTP 代理环境）：

```bash
A4N_PERF_EDGE_BEFORE=/path/to/baseline-edge \
A4N_PERF_EDGE_AFTER=/path/to/current-edge \
A4N_PERF_STARTUP_DIST=/path/to/production-dist \
  node e2e/performance/startup.mjs
node e2e/performance/schema-load.mjs
```

Edge 基线二进制由改码前的当前源码构建并保留；不能用同一个已优化二进制充当旧版本。上述路径必须在工作树或 `work/` 内；生产资源通过 `npm run build -- --outDir ... --manifest` 生成，已有部署脚本的 `--outDir` 调用方式保持有效。

## 完整目标仍在进行

以上不是整站所有性能目标已经完成的声明。首屏交付、核心计算、真实 Host 进程树、1/4/8 并发读请求、LLM TLS/SSE、CLI 写作与恢复链路、千章侧栏、本地及 API 嵌入和大型 JSON 交付已经验证。完整正文列表的载荷仍随章节数量增长，下一步需区分目录所需字段与按需读取的单章内容，不能用压缩后的相对提升宣称所有大项目已即时加载。

第三批已建立真实运行栈：Server、Control、Edge、CLI、迁移程序、30 个状态型 Worker 和 9 个能力服务，共 44 个冻结程序。使用独立 `a4n_perf_runtime_20260927` 数据库、数据目录和端口；实际进程树为 42 个进程、872 个线程。所有构建、配置与原始样本在 `data/performance-audit-20260927/b03/`。

## 第三、四批的验收证据

- **章节读取：** `b03/chapter-controlled-comparison.json` 与 `chapter-controlled-{1,2,3}-{before,after}/report.json` 是最终三轮交错样本。100 章、300 次运行、600 个检查点（JSON 合计 38,428,800 字节），响应 1,160,181 字节；新查询只返回 100 次最新运行及其 200 个检查点。所有章节响应的完整 canonical JSON 摘要均为 `1c26fe8ac1d1cc89bab5a243f13843ec5b4aec88cd27329bdd7b5e0fc5950473`。
- P95@1/@4/@8 分别为 **74.97 → 65.88 ms（12.12%）／125.60 → 67.25 ms（46.46%）／174.54 → 82.08 ms（52.98%）**。单并发时延没有达到 30%，不计作主要验收收益。Chapter Worker 读取后 PSS 中位数 772,316 → 171,726 KiB；整栈 PSS 指上述 42 个进程的同阶段合计，**不是峰值或空闲内存**。空闲 PSS 392,473 → 385,450 KiB，仅变化约 1.8%。未修改读口的少量毫秒级波动不计收益。
- 原始 35 个 package 的 feature 选择用于重建两个受影响程序，发布参数为 release、LTO=false；`chapter-matched-feature-differences.json` 的依赖 feature／profile 差异为空。`frozen-before` 与 `frozen-chapter-matched` 的其余 42 个程序保持逐字节相同，SHA-256 见对应 `binaries-*.json`。早期 `chapter-comparison.json` 存在构建选择和负载干扰，保留供复核，不作为最终时延结论。
- **模型传输：** `b03/llm-transport-comparison.json` 与 `llm-tls-final-{1,2,3}-{before,after}/report.json` 使用真实 Gateway、完整系统 CA 集加测试根证书、HTTPS 和 chunked SSE。前后依赖 feature／profile 一致，270 个探测响应的完整 JSON 摘要相同。每次更换模型名与测试 API key，服务端逐次核对，确认连接复用没有混用请求配置。首次五次作为预热，报告重复调用的中位数与 P95；实际模型推理耗时没有计入收益。
- 传输测试同时覆盖：1 秒读取超时下持续输出 1.8 秒成功、空闲 1.2 秒失败、改为 2 秒超时后成功、失败后继续请求成功。LLM HTTP **55** 项、Gateway **19** 项、独立进程 **11** 项通过；其中包含取消释放、背压、超时恢复、凭证转发边界。Client 数量上限与配置唯一性也有可执行检查。
- **写作侧栏：** `b04/sidebar-comparison.json` 使用生产 React／真实 Chromium 153.0.8010.52，进程及子进程固定 CPU 18、19，前后交错三轮，每轮预热后取五个样本中位数。两个版本使用同一夹具，只从 `e915d0373` 替换旧版 `ChapterSidebar` 和 `ChapterCard` 源码。蓝图／角色 API 返回固定空数据，章节行、计数、样式、React 和 DOM 均为生产实现。100／1000 章最后 DOM 摘要相同。
- 侧栏检查包含中英文、1440／390 宽度、emoji／组合字符／CRLF、空正文、新章本地草稿、服务器正文变更、过滤、键盘 Enter 选章，以及行数据不变时替换回调。原有切章回调沿用 `safeSelectChapter → confirmCurrentEdit`，没有绕过未保存编辑确认。生产构建、合同检查与 **432 项 Vitest** 通过；截图和每轮报告在 `b04/sidebar-{before,after}/`。
- **组合回归：** `b03/frozen-b03-final` 只替换 Control、Chapter Content Worker、LLM Gateway 三个程序。最终 `runtime-cli-b03-final` 的 CLI 写作 **13**、场景同步／恢复 **7**、模型失败／取消／预算 **8** 项通过；数据库最新记录选择 **2**、章节纯规则 **2** 项通过。Worker isolation 门禁在预编译后通过；此前的 60 秒编译超时保留为未通过记录，未冒充测试成功。
- 本批汇总为 `b03-b04-summary.json`，包含三个对照结果、生产源码指纹与组合 CLI 回归结果。夹具正向检查和历史数量不匹配时的拒绝检查均通过，见 `b03/fixture-validation.json`；失败检查在性能采样前退出。

复现入口：先显式设置 `A4N_PERF_DATABASE_URL`（本机 PostgreSQL、数据库名以 `a4n_perf_` 开头），并准备 ONNX Runtime 与 Qdrant。保持本工作树内的程序、数据和结果路径。脚本记录实际运行／检查点数量与 JSON 字节数；已有数据与 `--chapter-history` 不符时明确失败，避免把历史累积后的数据库误当成空白基线。

```bash
python3 scripts/performance/runtime-baseline.py \
  --bin-dir data/performance-audit-20260927/b03/frozen-b03-final \
  --state-dir data/performance-audit-20260927/b03/state \
  --report-dir data/performance-audit-20260927/b03/new-runtime-report \
  --chapter-history 3 --qdrant-url http://127.0.0.1:48973
python3 scripts/performance/llm-transport.py \
  --gateway data/performance-audit-20260927/b03/llm-gateway-production-after \
  --report-dir data/performance-audit-20260927/b03/new-tls-report
```

侧栏从 `frontend/` 执行前文相同的代理清理命令并加上 `A4N_PERF_CASE=sidebar`；用 `A4N_PERF_BASELINE_REF=e915d0373` 载入旧版两组件。先构建两份夹具，再设置 `A4N_PERF_REUSE_BUILD=1` 交错复测，输出目录由 `A4N_PERF_ARTIFACTS` 指定。

没有保留的资源调优：默认启动空闲 PSS 约 375 MiB，普通读取后约 490 MiB。`TOKIO_WORKER_THREADS=2/4` 和 `MALLOC_ARENA_MAX=2` 没有稳定达到本任务的内存／时延门槛；2 线程还使章节 P95@8 从约 38–43 ms 增至 58–61 ms。未修改产品线程数、allocator 或 `ServiceManager` 的 `env_clear()` 安全边界。

环境中断后，既有 PostgreSQL 容器已经恢复；Qdrant 使用本任务独立的 `b03/qdrant` 数据和 48972／48973 端口。基准脚本支持 `--qdrant-url`。`--cli-checks` 会在读取测量结束后运行已有三套 CLI 测试，应使用独立的一次性数据库；它创建的业务夹具不能混入后续读性能对照。首次 CLI 场景测试因导入默认 384 维与后续 8 维模型不一致失败，原始错误和 RAG 保护标记保留；修正测试配置后使用新隔离库验证，未改产品校验。

## 第五、六批：嵌入资源生命周期

- 本地模型为应用默认推荐的 `BGESmallZHV15`（`Xenova/bge-small-zh-v1.5`，512 维，约 90.9 MiB）。使用现有显式下载 API 获取到本任务的 `b05/state/models`，模型权重 SHA-256 为 `69a0b846f4f116b5e6aabf9546ea6754d02264f3211a13a1bd69b31b8040749a`。所有对照使用相同权重、输入和 release／依赖 feature 配置；模型下载时间不计入推理收益。
- `b05/comparison.json` 是三轮交错、真实 ONNX 模型的完整 RPC 对照。1 条文本在 1／4 并发下的中位耗时分别 **428.07 → 34.72 ms／1257.28 → 153.06 ms**；32 条分别 **678.17 → 344.87 ms／2340.93 → 1268.56 ms**。所有身份字段与每个向量元素完全相同，包括额外的批量大小 1／8／32 对照。
- 单个 embedding 进程的负载后 RSS 为 **424,632 → 229,788 KiB**，`VmHWM` 峰值为 **599,124 → 229,788 KiB**。结束时线程数从 44 变为 62：已加载 ONNX 会话的工作线程随模型保留，不能宣称线程数也下降。该实现只额外保留一个配置，已有长寿命 provider 继续持有自己的模型；混合模型持续切换时可能重新加载，这是有界内存策略的已知取舍。
- `b06/native-idle-{before,after}` 在负载结束并稳定后取样，两版空闲 CPU 均为 0%；最终组合程序的向量仍与原版逐值相同。扩展传输夹具后，原有 LLM 模式的四项超时／恢复检查再次通过。汇总及生产源文件指纹见 `b05-b06-summary.json`。
- 初始化和推理继续使用原来的阻塞任务及错误映射。模型锁随阻塞任务持有，调用方断开不会提前释放锁、制造第二份正在初始化的模型；推理错误仍归还模型，线程 panic 不留下半完成模型。每次使用前仍检查下载文件，未引入隐式下载。
- 文件回归先成功推理，再经真实下载／删除 API 变更生命周期。测试临时原子替换本任务的 ONNX 文件为损坏文件，确认返回初始化错误而不是旧模型结果；随后恢复原始权重并确认完整向量一致。还验证未知模型／维度／批量参数拒绝、请求在途时断开、后续请求恢复。恢复操作位于 `finally`，原始文件摘要再次核对。回归在原版和修改版均通过。
- `b06/http-comparison.json` 使用真实 Embedding Service、完整系统证书集和 HTTPS 夹具；1／32 条文本各三轮、每轮 45 个请求，逐次更换模型和凭证，540 个完整响应按请求序号一致。500 错误、维度不匹配和错误后的正常请求保持原行为，固定 60 秒总超时未改变。此项测量的是 API 传输及本地处理，未包含外部模型推理时间。
- FastEmbed／Embedding Service／进程回归 **22 项通过**，embedding providers **4 项通过**。首次测试链接缺少 `ORT_LIB_PATH` 的失败保留；配置项目使用的 ONNX Runtime 1.24.2 动态链接路径后编译和测试通过。最终两个分支使用相同依赖 feature／profile，差异文件为空。
- `b06/frozen-final` 仅在上一批冻结程序基础上替换 Embedding Service。组合后的 `b06/cli-final` 写作 13、场景恢复 7、模型失败与取消 8 项通过。`b06/scale-1000-recheck` 验证 1000 章顺序和完整正文，响应 **11,601,686 字节**，单客户端 P50／P95 **86.73／103.30 ms**，读取后整栈 PSS **606,041 KiB**。这是本机传输结果，不是低带宽 Web 结果。

复现入口：

```bash
python3 scripts/performance/embedding-lifecycle.py \
  --service data/performance-audit-20260927/b06/embedding-service-after \
  --state-dir data/performance-audit-20260927/b05/state \
  --report-dir data/performance-audit-20260927/b05/new-report --verify-files
python3 scripts/performance/llm-transport.py \
  --gateway data/performance-audit-20260927/b06/embedding-service-after \
  --kind embedding --texts 32 \
  --report-dir data/performance-audit-20260927/b06/new-http-report
```

首次准备模型时显式加 `--download`。文件生命周期测试只允许使用本任务独立的模型缓存；结果目录必须是新的。`runtime-baseline.py --chapter-count 1000 --concurrency 1` 可复现大章节读取，其余夹具和隔离数据库要求不变。

## 第七批：Web JSON 交付

- `b07/comparison/report.json` 与 `b07/summary.json` 使用真实旧／新 Edge、同一 Control 夹具、实际生产登录页和 Chromium。语料来自项目已有 Markdown 提示词，整体超过 gzip 窗口，避免重复一个短句来制造压缩率。每组载荷在 4 Mbps 下载、50 ms 延迟下前后交错三轮。
- 1,131,054 字节 JSON 的传输字节中位数降到 495,776；11,027,654 字节降到 4,831,316。两组耗时分别下降 53.99%／55.79%。这项收益来自减少传输字节，动态压缩本身使用 CPU；没有缓存用户数据、改变字段或降低刷新频率。
- 只压缩大于 1024 字节的成功 JSON，保留原有 `Vary`；ETag 和 `Cache-Control: no-transform` 响应保持原样。原生中间件也拒绝重复压缩及 Range 响应。真实 HTTP 检查覆盖身份表示、gzip、`gzip;q=0`、HEAD、Range、已编码响应、小 JSON、ETag、no-transform 和授权头转发。
- SSE 没有进入 JSON 压缩层；测试首段约 1.2 ms 到达、尾段约 801.5 ms 到达。静态资源沿用 PERF-009 的构建期 gzip，不叠加动态压缩。
- Edge 原有 5 项测试、实际浏览器及协议检查通过。`b07/cli-final` 使用只替换 Edge 的最终冻结程序完成 28 项 CLI 业务回归。真实千章 API 在身份／gzip 协商下的完整 JSON 相同；这批合成正文高度重复，真实 API 的压缩率只作协议验证，不作为主要收益依据。
- 本次仅为已有 `tower-http` 启用 `compression-gzip`，锁文件新增其 3 个编解码依赖，既有依赖版本未更新。没有自建压缩协议或业务缓存。
- 浏览器基准首次因测试临时目录过长触及 Unix socket 长度限制而启动失败，见 `b07/startup-diagnostic/driver-verbose.log`；缩短本任务临时目录前缀后通过，没有修改用户浏览器设置或关闭其他会话进程。

复现：从 `frontend/` 执行前文的代理环境清理，再运行：

```bash
A4N_PERF_EDGE_AFTER=/path/to/new-edge \
A4N_PERF_ARTIFACTS=/path/inside/worktree/data/api-transfer \
  node e2e/performance/api-transfer.mjs
```

`A4N_PERF_EDGE_BEFORE` 可指定旧版 Edge；默认使用本任务 `b06/frozen-final/a4n-edge`。测试读取同一份已冻结的生产前端资源，并逐次核对解压结果摘要。

### PERF-017：目录与章节正文分开读取

- 请求位置：写作桌初载/刷新、小说只读目录和导入匹配预览。旧目录接口传输所有章节正文、修订及分析；千章真实响应为 11,601,686 字节。目录使用新的 `list_chapter_index`，正文使用带项目归属的 `get_chapter`。保留原完整 API；分析/资料状态共用原投影，Unicode 正文字数与存储字数分别保留。
- 前端只持有当前章完整内容；切章迟到响应不能覆盖后来选择或写后结果；加载完成前不开启正文编辑/草稿恢复。保存、生成结果核验及优化完成后的刷新可显式指定目标 chapter ID。上一章结尾在展开上下文时读取。漫画、视频和 Galgame 目录使用窄章节索引，选章后再读取正文；故事事件页面仍使用分析投影。
- 真实 PG/Server/Control/Worker：`b08/runtime-02/report.json` 验证千章全部目录字段与原完整响应逐值相同，目录 742,686 字节（减少 93.60%）；抽查首/中/末章完整响应相同，错误项目 404、非法项目 ID 400、无认证 401。这个阶段未减少数据库正文读取，不宣称相同幅度的 DB 内存收益。
- 同 1000 章自然语料、4 Mbps/50 ms、三轮浏览器候选值：旧版本可编辑时间中位 5264.6 ms，候选 741.1 ms（下降 85.92%）。证据 `b08/browser-before-final` / `b08/browser-final-02`。全部 API 压缩传输字节 2,090,160 → 15,347。浏览器交付测量使用生产组件和 HTTP/gzip 夹具；真实后端合同验证独立执行，不把夹具时间当真实数据库端到端时间。
- 已验证：API 合同 106 项、Core 2 项、最终前端 445 项、当前前端类型检查与生产构建、组合 CLI Mock Provider 28 项；`browser-final-02` 通过切章取消、草稿归属、草稿写入失败阻止离开、上一章按需上下文、迟到读取、失败读取、保存失败/重试、保存期间继续编辑、延迟读取后的恢复草稿、只读预览和 Escape，1440/390px 无横向溢出。
- 后续 `690900a58` / `a97aa1af7` checkpoint 合入导航保护后，`browser-after-02` 曾在已有切章确认之后再次触发全局放弃草稿对话框。已收敛为现有路由保护处理导航，删除写作桌按钮里的重复确认；刷新正文仍有独立确认。本地恢复副本随切章保留，写草稿失败时明确报错并留在当前章。最终浏览器已通过。此前失败报告保留，checkpoint 本身的 `A4N-Ready: no` 不当作本项完成标记。
- **最终门禁：** 类型已重新生成，`check:web-api-parity`（包括2387组生成事件校验对照）通过；受影响 crate 及全仓 rustfmt、diff 检查通过。全仓格式检查先前因 Import Service 测试参数行的多余缩进失败，现只修正该行缩进，未改变测试输入或逻辑；失败日志保留。
- 真实测试使用 `b08/frozen-candidate` 的 44 个程序，其中 Server/Control/CLI/Chapter Content Worker 为本阶段构建。迁移程序从同源码的独立打包基线复用，448 个本地依赖源码文件逐一一致，来源和哈希见 `b08/migration-provenance.json`。初轮旧迁移程序因缺少 `media_authoring` 失败，日志保留。

浏览器复现：从 `frontend` 执行 `node e2e/performance/chapter-load.mjs`；`A4N_PERF_BASELINE_REF=0a3dff98a` 选择旧读法，`A4N_PERF_ARTIFACTS` 指向独立证据目录。真实服务读取验证在原 `runtime-baseline.py` 命令上增加 `--chapter-index-checks`；CLI 测试必须使用另一隔离数据库。

### PERF-018：故事事件筛选的重复渲染

`filterStorySections` 保留所有事件时直接沿用原章节对象；筛掉部分事件时仍产生新对象，规则、顺序、文本和计数不变。`ChapterEventSection` 使用 React 默认浅比较，并保留动态回调和 i18n context 更新。没有新增结果缓存、搜索索引或延迟输入。

证据为 `data/performance-audit-20260927/b09/summary.json`，包含源码指纹。`before-1..3` / `after-1..3` 固定 CPU18/19 交错运行，每轮预热两次后取五次样本中位数。100／1000 章分别有 500／5000 个事件；八次查询包含共同前缀、特定事件收窄和放宽。结果是这八次同步过滤与 React 提交的总耗时，不是首次加载或后端延时。六轮完整 DOM 摘要及中英文、1440/390px 筛选追踪相同；替换打开章节回调后 Enter 正确调用新回调，无页面横向溢出。最终 445 项前端测试和 TypeScript/Vite 构建通过。

复现：从 `frontend` 执行 `A4N_PERF_CASE=story-events A4N_PERF_ARTIFACTS=<独立目录> taskset -c 18,19 node e2e/performance/check.mjs`；旧版加 `A4N_PERF_BASELINE_REF=ff9fb6823`。只替换两个被测生产模块，其他夹具和依赖相同。

### PERF-019：故事事件的五列读取

`ChapterAnalysisEntry` 在 Domain 定义，由 API 合同直接重导出；DB 查询绑定 project_id，沿用 `find_current()` 和章节号排序。新增 `list_chapter_analyses` 经过现有章节读端口，Chapter Content Worker 合同升级为 v5；原完整章节接口保持不变。前端事件提取、筛选和显示规则未改变，仅更换读取合同。不请求或构造页面未使用的正文、修订及同步状态。

- `b10/runtime-01..03/report.json`：同一套真实 Server/Control/Worker/PG 上，分别测原完整接口与新投影，每个接口35次请求、前5次预热；最终取三轮P50/P95中位数。1000章中100章无分析，另有软删除章与其他项目章用于边界验证。全部五列逐值一致，删除/项目隔离、401/400/403拒绝及viewer可读均通过。
- `b10/browser-before-checked` / `browser-after-checked`：生产 StoryEventsTab 渲染真实API返回的合成夹具；六次页面完整文本摘要一致，4500事件全部呈现，错误可见、刷新恢复、1440/390px无横向溢出。4Mbps/50ms下页面中位 **1948.8 → 1704.8 ms（约12.5%）**，未达到30%，不作为本项主要验收收益；本项验收依据为真实API的65.76%/58.68%时延改善。
- 最终API106项/Worker合同2项、前端445项、类型/Vite构建、web-api-parity及2387组校验对照、全仓rustfmt/diff通过。`cli-final-02` 的core16/scene7/llm8共 **31项** 通过，新增场景实际调用目录、项目作用域单章及分析投影。
- 所有失败均保留：首次发布构建遇到并行健康检查接口尚未补齐；首次CLI的旧Import sidecar缺少新的解析路由；随后一轮错误复用了已被CLI写入的测试库。已重编译对应程序、替换Import/Export/Media sidecar，并按CLI运行分配新数据库。浏览器恢复测试曾过早把“错误暂时隐藏于加载状态”当完成，现同时等待全部事件恢复。

证据目录：`data/performance-audit-20260927/b10/`，最终程序与哈希在 `frozen-candidate` / `binaries-final.json`。真实基准在 `runtime-baseline.py` 增加 `--chapter-analysis-checks`；它准备专用合成数据并与旧接口逐值对比。浏览器使用 `A4N_PERF_VIEW=story`、`A4N_PERF_CHAPTER_FIXTURE=<该运行的chapter-fixture.json>` 调用 `chapter-load.mjs`；旧版加 `A4N_PERF_BASELINE_REF=ecd9fb5b7`。

### PERF-020：派生媒体源快照按目录和按需正文读取

- 请求链路：Comic、Video、Galgame 工作区初载只需要章节号/标题/快照章节主键；漫画/视频选章和 Galgame 场景详情才需要正文与分析。旧路径把 1000 章正文、摘要和分析放进一次目录响应，并在漫画初载读取全部媒体编排源跨度。新路径增加窄目录和带 `project_id + snapshot_id + source_chapter_id` 校验的单章详情，保留旧完整读取命令供仍需全量数据的调用方使用。
- 后端范围读取：全量媒体编排改为在已验证根项目和快照范围内一次读取实际使用的字段，避免完整快照摘要物化后再次查询正文；正文哈希校验、章节顺序和错误边界保持不变。该路径只由媒体编排使用，目录接口仍使用窄索引。
- PostgreSQL 千章基准 `data/performance-audit-20261010/`，15 轮交错、2 轮预热、每章约 9.2 KiB 正文：完整列表 P50/P95 **321.991/332.893 ms、12,463,579 B**；目录 + 首章详情 **10.919/14.512 ms、111,034 B**，P50 下降 **96.61%**，响应体下降 **99.11%**。全量媒体编排旧路径 **270.900/297.334 ms**，单次范围读取 **170.880/189.660 ms**，P50 下降 **36.92%**；返回章节数量、顺序和正文总量一致。
- 真实 Chromium 153.0.8010.52 夹具覆盖 1000 章、1440/1024/768/390/375/320px、Enter 选章和无横向溢出。初次可用时间 **312.4 → 125.1 ms（59.97%）**，请求 JSON **24,861,598 → 122,722 B（99.51%）**，gzip 传输字节 **133,485 → 12,009 B（91.01%）**，JS heap **26.38 → 7.29 MB（72.38%）**。优化后初始页面不读取正文；Enter 选章仍获得单章详情，旧/新 DOM 行数均为 1000。
- 目录与详情功能测试覆盖当前快照、跨项目、过期快照、非法参数和正文投影；`a4n-db` 源快照功能测试 **1 passed**，千章 A/B 基准 **1 passed**。前端 Vitest **489 passed**、TypeScript、生产构建和 `check:web-api-parity` 通过；Chromium 夹具证据为 `data/performance-audit-20261010/manga-before-02/` 与 `manga-after/`。首次数据库夹具因本机缺少 `postgres:16` 镜像拉取超时，改用本机独立 `a4n_perf_20261010` 数据库，失败日志保留。

复现：

```bash
export A4N_TEST_DATABASE_URL=postgres://a4n:a4n@127.0.0.1:5432/a4n_perf_20261010
CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target \
  cargo test -p a4n-db --test source_snapshot_queries \
  performance_source_snapshot_reads_1000_chapters -- --ignored --nocapture
cd frontend
A4N_PERF_TMPDIR=/home/fufu/Code/A4N/work/.pt \
A4N_PERF_CASE=manga-source \
A4N_PERF_BASELINE_REF=3f6a2408c \
A4N_PERF_ARTIFACTS=../data/performance-audit-20261010/manga-before-02 \
  node e2e/performance/check.mjs
```

### PERF-021：Galgame 工作区延迟加载媒体编排范围

- 请求位置：Galgame 工作区默认 `materials` 阶段。旧实现挂载 `GalgameTab` 即请求 `get_media_authoring_workspace`，该响应包含整个源快照的章节跨度；材料页只显示角色/素材，不读取创作工作区。新实现把读取开关绑定到 `adaptation`/`branches` 阶段，进入这两个阶段后仍按原命令读取并保留错误/权限处理。
- 真实 Chromium 153.0.8010.52 夹具使用 1000 个源章节、4 Mbps/50 ms 传输模型，前后版本只替换 `GalgameTab.tsx`，三轮交错取中位数。初始请求 JSON **9,460,572 → 20 B（99.9998%）**，gzip 字节 **46,854 → 20 B（99.96%）**；初始可用时间 **226.8 → 154.0 ms（32.10%）**；JS heap **30.30 → 23.96 MB（20.94%）**。DOM、章节计数和 780px 窄屏宽度保持一致。
- 夹具随后点击 `adaptation` tab，候选版本确认发出且完成一次 `get_media_authoring_workspace`，证明延迟读取没有删除工作流。证据为 `data/performance-audit-20261010/galgame-before-{02,03,04}/` 与 `galgame-after-{05,06,07}/`；复现命令沿用 `A4N_PERF_CASE=galgame-source` 和 `A4N_PERF_BASELINE_REF=3f6a2408c`。

### PERF-022：Galgame 脚本与资产计划按阶段读取

- 请求位置：Galgame `materials`、`adaptation`、`branches` 阶段不使用完整脚本和资产计划；`script`/`export` 阶段需要脚本，`asset_plan`/`asset_generation` 阶段需要脚本和资产计划。旧实现三类数据在所有阶段挂载时并行读取；新实现按阶段显式投影，刷新和材料页“下一阶段”仍可强制完整刷新。
- 真实 Chromium 153.0.8010.52 夹具生成 1000 个脚本场景、1000 个资产规格和 1000 个源章节，三轮交错取中位数：初始可用 **303.7 → 138.4 ms（54.45%）**，初始 JSON **10,616,545 → 12 B（99.9999%）**，gzip 字节 **90,431 → 12 B（99.99%）**，JS heap **33.52 → 26.37 MB（21.34%）**。DOM、章节计数和窄屏宽度保持一致。
- 同一夹具随后依次点击 `adaptation`、`script`、`asset_plan`：候选版本分别完成一次媒体编排读取、一次脚本读取，以及第二次脚本读取加一次资产计划读取；因此延迟加载没有把后续业务功能变成空状态。证据为 `data/performance-audit-20261010/galgame-script-before-{02,03,04}/` 与 `galgame-script-after-{06,07,08}/`，入口为 `A4N_PERF_CASE=galgame-source`。

### PERF-023：媒体编排章节级预处理产物批量读取

- 请求链路：漫画媒体编排 workspace 为每份 adaptation plan 判断预处理产物是否过期。旧路径在文档循环内按章节逐条查询，并且每份文档重复读取角色立绘资产；新路径预取所有源章节的章节级 preprocessing 产物和一次角色资产快照，在内存中完成每份文档的 fingerprint 判断。章节身份、阶段、页码范围和 stale 规则保持不变。
- 数据量与频率：独立 PostgreSQL 中 1000 个章节、1000 条 preprocessing 产物，15 轮交错测量，2 轮预热；旧路径每轮 1000 次数据库 round-trip，新路径每轮 1 次 `IN` 查询。新查询返回 `HashMap<chapter_id, record>`，缺失产物仍按原规则判定过期。
- A/B 结果（debug 测试目标）：旧路径 P50/P95 **1012.415/1460.258 ms**，新路径 **18.177/32.348 ms**；P50 下降 **98.20%**，P95 下降 **97.78%**。1000 条记录逐字段比较一致，批量结果数量与章节集合一致，满足至少 30% 时延收益门槛。
- 验证：`a4n-db` 批量基准 1 passed；`a4n-db` 与 `a4n-orchestrator` `cargo check --locked`、格式和 diff 检查通过。首次 release 基准因冷编译超过 60 秒门禁未产出测试程序，未计入性能数据；debug 目标完成同一基准并保留该失败记录。

复现：使用独立 PostgreSQL 数据库并在 agent 构建锁内执行：

```bash
export A4N_TEST_DATABASE_URL=postgres://a4n:a4n@127.0.0.1:5432/a4n_perf_20261010_manga_batch
flock -x /home/fufu/Code/A4N/work/.agent-build.lock -c \
  'CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 \
   cargo test -p a4n-db --test manga_pipeline_batch \
   performance_manga_pipeline_artifacts_batch_1000_chapters -- --ignored --nocapture'
```

### PERF-024：媒体编排视频参考状态批量读取

- 请求链路：workspace 为每份带视频参考的文档计算 `VideoReferenceStatus`。旧路径对每份文档重复验证视频项目 viewer 权限并读取对应剧本；新路径先批量筛选可见视频项目，再按 `(project_id, script_id)` 一次读取全部剧本，随后在内存中比较 revision 和脚本哈希。无参考、无权限、缺失剧本和脚本更新仍分别映射为原来的 `None`、`Unavailable` 或 `Updated`。
- 数据量与频率：独立 PostgreSQL 中 1000 个视频剧本引用，15 轮交错测量，2 轮预热。旧路径每轮 2000 次数据库 round-trip；新路径为一次权限批量查询和一次剧本 `IN` 查询。
- A/B 结果：旧路径 P50/P95 **1829.005/4209.135 ms**，新路径 **21.136/31.158 ms**；P50 下降 **98.84%**，P95 下降 **99.26%**。1000 条剧本记录逐字段比较一致，批量权限结果与逐项 owner 权限一致，满足至少 30% 时延收益门槛。
- 验证：`a4n-db` 视频引用批量基准 1 passed；`a4n-db` 与 `a4n-orchestrator` `cargo check --locked`、全仓 rustfmt 和 diff 检查通过。基准只使用独立数据库/schema，不读取或写入开发数据。

复现：

```bash
export A4N_TEST_DATABASE_URL=postgres://a4n:a4n@127.0.0.1:5432/a4n_perf_20261010_manga_batch
flock -x /home/fufu/Code/A4N/work/.agent-build.lock -c \
  'CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 \
   cargo test -p a4n-db --test video_reference_batch \
   performance_video_reference_status_batch_1000_scripts -- --ignored --nocapture'
```

### PERF-025：视频参考列表的源快照批量解析

- 请求链路：`video_references` 列表需要校验每个候选视频剧本冻结的源章节。旧路径对每个剧本单独读取并校验快照正文，同时逐项验证权限；新路径复用批量项目权限结果，按 `(video_project_id, root_project_id, snapshot_id)` 分组，一次读取该组全部源章节，并用章节索引替代源列表线性查找。候选顺序、权限过滤、源快照身份和内容哈希校验保持不变。
- 数据量与频率：独立 PostgreSQL 中 1000 个冻结源章节，15 轮交错测量、2 轮预热；旧路径每轮 1000 次快照查询，新路径每轮 1 次批量查询。逐条 `ResolvedChapterSource` 完整字段一致。
- A/B 结果：逐条解析 P50/P95 **713.831/2151.846 ms**，批量解析 **45.791/55.295 ms**；P50 下降 **93.59%**，P95 下降 **97.43%**，超过 30% 时延门槛。
- 验证：`a4n-db` 源解析批量基准 1 passed；`a4n-orchestrator` 受影响 crate check 通过。基准只使用独立 PostgreSQL schema。

复现：

```bash
export A4N_TEST_DATABASE_URL=postgres://a4n:a4n@127.0.0.1:5432/a4n_perf_20261010_manga_batch
flock -x /home/fufu/Code/A4N/work/.agent-build.lock -c \
  'CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 \
   cargo test -p a4n-db --test video_reference_source_batch \
   performance_video_reference_source_resolution_batch_1000_chapters -- --ignored --nocapture'
```

### PERF-026：视频参考列表窄脚本投影

- 请求链路：`video_references` 只需要候选项目 ID/标题、剧本身份、源章节身份、revision 和哈希；旧查询把完整项目行和 `script_json` 读入模型后再丢弃，新查询对项目只选 ID/标题、对剧本只选 10 个身份列，并使用 `ChapterShootingScriptReferenceSummary` 领域投影。完整脚本 JSON 仍由单剧本冻结流程按需读取。
- 数据量与频率：独立 PostgreSQL 中 1000 个候选剧本，每条 JSON 约 64 KiB，15 轮交错测量、2 轮预热；新旧结果逐字段核对。
- A/B 结果（最终项目/脚本窄投影代码）：全量 ORM 读取 P50/P95 **391.134/438.989 ms**，窄投影 **7.161/9.104 ms**；P50 下降 **98.17%**，P95 下降 **97.93%**。旧路径一次物化约 **65,596,000 B** 脚本 JSON，并读取/重复克隆完整项目记录；新路径不物化脚本 JSON，项目只读 ID/标题，超过时延和内存载荷门槛。
- 验证：`a4n-db` 窄投影基准 1 passed；`cargo check --tests -p a4n-db -p a4n-orchestrator --locked`、格式和 diff 检查通过。

复现：

```bash
export A4N_TEST_DATABASE_URL=postgres://a4n:a4n@127.0.0.1:5432/a4n_perf_20261010_manga_batch
flock -x /home/fufu/Code/A4N/work/.agent-build.lock -c \
  'CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 \
   cargo test -p a4n-db --test video_reference_projection_batch \
   performance_video_reference_projection_1000_large_scripts -- --ignored --nocapture'
```

### 拒绝候选：PERF-027 故事事件列表渲染

- 目标链路：故事事件页面 1000 章/5000 事件的筛选更新与初次卡片渲染。分段 Chromium 基准显示初次 React commit 约 **167.6 ms**，章节/事件数据转换约 **24.6 ms**，筛选八次更新主要耗在卡片协调；初始 DOM 约 **69,008 个元素**。
- 候选 A：移除每个事件行的装饰 bullet 和分隔符 DOM，改用 CSS 伪元素。DOM 降至 **49,008**，完整 HTML trace、1440/390px 横向溢出检查和键盘路径保持一致；但八次筛选总耗时 P50 仅 **177.1 → 154.0 ms（13.0%）**，低于 30% 门槛，拒绝保留。
- 候选 B：对章节数组做自定义引用比较以跳过列表协调。实际 Chromium P50 **667.3 ms**，相对基线反而约 **3.77 倍**，立即撤回。
- 结论：两项产品改动均未进入源码；仅保留性能夹具中的分段计时、DOM 计数和完整 DOM trace，用于后续评估真正的列表窗口化方案。当前实现仍保留完整 DOM、查找、键盘和打印语义。

### 拒绝候选：PERF-028 故事事件搜索筛选

- 链路与规模：搜索输入触发 `filterStorySections`，扫描 1000 章/5000 个事件；其后 React 列表协调占八次交互的大部分时间。测量使用 Chromium 153、1440/390px，固定源码基线 `01bcafc8d`，文本/HTML 哈希、事件数、Enter 跳章和横向溢出一致。
- 候选 A：在章节/事件上预计算规范化搜索字符串。筛选阶段 P50 `27.9 → 3.4 ms`（下降 87.8%），但整段八次交互 `171.0 → 171.7 ms`，初始 heap `28.9 → 35.0 MB`（增加 21.2%）；拒绝。
- 候选 B：章节文本命中时跳过事件扫描、输出未变时复用数组并 memo 章节列表。一次新鲜 A/B 的筛选阶段 P50 `27.9 → 5.8 ms`（下降 79.2%），但整段交互 P50 `184.2 → 159.7 ms`（下降 13.3%），仍低于 30% 门槛；产品改动撤回。heap 读数受浏览器运行波动影响，不作为收益证据。
- 结论：两种搜索候选均未保留。证据在 `data/performance-audit-20261010/story-search-before-01/report.json`、`story-search-after-02/report.json` 和 `story-search-memo/`。夹具保留分段计时、DOM/HTML/渲染文本校验与 heap 观测，供后续端到端列表候选复用。
- 夹具临时 profile 默认放在 `/home/fufu/Code/A4N/work/.p`，保持在 work 管理目录内，并避免深层工作树路径导致 Chromium socket 超长。
- 恢复候选后的基线复核：`story-search-final/{before,after}/report.json` 的 1000 章完整 HTML 与渲染文本 SHA-256 一致。单轮中位数 `355.8/289.5 ms` 有明显运行波动，产品源码没有符合因果归属的性能差异，因此不计入任何优化收益。

### PERF-029：Agent 外部等待恢复批量读取

- 链路与频率：Agent worker 的维护扫描每 **2 秒**运行一次；`task_wait::reconcile` 最多分页读取 **32 个**外部任务等待项，再从 DB 读取 Thread、Execution 与每个 Run 的预算快照。旧实现每次最多执行 **161 条查询**（等待 Run 1 条，Thread/Execution 各 32 条，预算快照每项 3 条）。
- 根因与改动：列表读取改为一次批量 Thread 和一次批量 Execution 查询；新增 `run_budget_snapshots`，每个扫描批次只读一次目标 Run、一次所有目标根的 Run 家族、一次家族 Execution，再在内存中沿原有祖先链规则计算预算。单项 `run_budget_snapshot` 委托同一批量实现，保留唯一业务计算；应用层批量调用并在缺少结果时显式报错。空等待列表仍在首条查询后早返回。
- 量化结果：隔离 PostgreSQL 16、32 个真实等待 Run、2 次预热、15 次热样本。DB 扫描 P50/P95 **67.503/89.189 → 7.913/17.294 ms**，分别下降 **88.3%/80.6%**；SQL 数 **161 → 6（下降96.3%）**。数据量和调用频率符合维护扫描上限；未更改等待状态、游标、执行身份、预算限制或唤醒条件。
- 语义验证：基准逐项比较批量等待记录与 `external_task_wait`，并逐项比较批量预算快照与单项快照；父/子/兄弟 Run 共用预算树也逐项对照一致。预算回归 **5/5**、外部等待身份及唤醒 **1/1** 通过；`a4n-db` 与 `a4n-agent-application` 测试目标编译通过。
- 证据：最终复测 `data/performance-audit-20261010/agent-waits-final/command.log`、`database.json`；先前完整 A/B `agent-waits-before-final/command.log` 与最终运行均通过。初次默认镜像标签拉取失败、冷编译超过单测 60 秒窗口及一次报告目录参数错误均保留报告；最终使用已存在的镜像 `docker.m.daocloud.io/library/postgres:16` 和隔离临时库，编译后以 `timeout 60s` 单独运行测试。临时容器退出，agent target 已清理。

### PERF-030：Generation 取消候选批量预算读取

- 链路与频率：Agent worker 每 **2 秒**调用一次 `cancel_stopped_owned_tasks`；Generation 候选按 task ID 分页、每页最多 **32 项**。旧实现每项读取 owner/attempt，再读取 owner 状态和完整预算事实；32 个活动 root owner 的基线为 **193 条 SQL**。
- 根因与改动：owner/attempt 校验仍逐候选解析；祖先状态改为一次递归 CTE 批量读取，在内存中保持停止、循环和缺失祖先检查次序；仅对仍活动 owner 调用共用 `run_budget_snapshots`。停止候选仍早返回且不解析预算。
- 量化结果：隔离 PostgreSQL 16、32 个真实 pending Generation task 及活动 Agent owner，2 次预热、15 次交错样本。完整候选扫描 SQL **193 → 69**；最终同轮 A/B P50 **87.297 → 39.672 ms（54.5%）**，P95 **110.532 → 42.642 ms（61.4%）**。新旧候选 task/attempt/user/status/owner-stop tuple 逐项一致。
- 业务回归：Generation attempt 所有权、任务终态费用回执定向测试 **1 passed**；父/子/兄弟 Run 的状态批量判定在取消前后分别与期望状态一致；`a4n-db` 与 `a4n-agent-application` 测试目标编译通过。证据：`generation-cancel-before/command.log` 保存基线，`agent-cancel-batched-status-retry/command.log` 保存批量状态读取 A/B，`agent-cancel-final/command.log` 保存去重单项 owner 查询后的测试。

### PERF-031：Workflow 取消候选批量预算读取

- 链路与频率：与 Generation 扫描同属 Agent worker 每 **2 秒**维护任务；Workflow owner 依据工作流 ID 和获批的 `run_canvas_graph`/`run_canvas_node` execution 身份确定，每页最多 **32 项**。旧实现每候选一次 owner 查找，再逐项查 owner、祖先 Run 与预算树；32 个活动 root owner 约 **161 条 SQL**。
- 根因与改动：Workflow owner 身份查询仍逐候选解析；祖先状态改为同一批递归 CTE 读取，之后只为活动 owner 批量计算预算。
- 量化结果：隔离 PostgreSQL 16、32 个真实未完成 Workflow 与有效 Agent start execution、2 次预热、15 次交错样本。候选 tuple（workflow ID、owner ID、stopped）逐项一致；SQL **161 → 37**，P50 **186.456 → 54.368 ms（70.8%）**，P95 **319.796 → 87.222 ms（72.7%）**。
- 验证：最终同库交错报告 `data/performance-audit-20261010/agent-cancel-batched-status-retry/command.log` 中，Generation/Workflow 两项基准、父子状态检查、Generation 所有权、等待唤醒与预算回归均通过。中间单项状态候选已被批量祖先 CTE 取代，其数据不作为最终收益。

### 拒绝候选：PERF-033 编辑器等待过期预读

- 链路与规模：Agent worker 每 2 秒检查过期编辑器等待，TTL 为 **30 秒**，单次上限 32 个。每项候选需重查最近 `run_waiting` 请求与 Thread owner，随后在事务内按 thread → run → event → execution 锁序复查并转换状态。
- 候选：把候选请求/Thread owner 预读放进同一条有界查询，并将扫描时读取的 thread ID 传入事务前置函数，事务内部锁定和状态/执行身份复查不变。
- 量化结果：隔离 PostgreSQL 16、32 个真实编辑器等待、2 次预热、15 次样本；完整过期转换 P50/P95 **458.219/652.326 → 355.461/458.863 ms**，下降 **22.4%/29.7%**，未达到任一 30% 门槛。候选版本没有保留：P50 无阈值收益，且尾延迟只由单组样本测得。
- 验证与证据：前后均使 32 个等待执行进入失败终态；`editor_wait_is_fenced_and_expires_without_dispatching` **1 passed**。报告 `data/performance-audit-20261010/editor-expiry-{before,after}/command.log`。`runs.rs` 产品改动已撤回；仅保留可重放夹具和拒绝记录。

### 拒绝候选：PERF-032 审批过期预读

- 链路与规模：Agent worker 每 2 秒扫描一次过期审批，单次上限 32 项；每项须按 thread → run → execution → approval 顺序锁定并事务化更新多个状态及追加事件。隔离 PostgreSQL 的真实 32 项完整过期流程 P50/P95 为 **256.884/489.065 ms**。
- 候选：移除事务前重复的 Run identity 读取，把 approval/thread 与 Run 一致性检查移动到已锁定的 Run。另一次同库重复流程 P50/P95 为 **257.446/341.937 ms**：P50 无改善，P95 单次约下降 30.1%，未证明稳定满足门槛，且候选改变了不一致数据的校验时机；产品实现已撤回。
- 结论：保留基准夹具与拒绝记录，不保留产品变更。证据 `data/performance-audit-20261010/approval-expiry-{before,after-candidate}/command.log`；两次过期转换均 **32 项全部 expired**，另 `expired_approval_rejects_execution_and_requeues_run` 与 `approval_is_consumed_only_after_execution_succeeds` 各 1 项通过。

### PERF-034：expired-run recovery 基线

- 链路与频率：Agent worker 每 **2 秒**扫描一次，最多取 **16 个**过期 lease；应用层顺序调用每个 Run 的恢复事务。恢复需原子完成状态 fencing、执行/流消息终态化、审计事件及 Thread 状态投影。
- 量化结果：隔离 PostgreSQL 16、16 个真实过期 root Run、2 次预热、15 个样本；包含候选扫描和全部恢复事务的批次 P50/P95 **111.002/263.408 ms**，约占 2 秒维护周期 **5.6%/13.2%**。16 个 Run 均进入 `recovery_required`。
- 决定：本轮不改逐 Run 事务。批量跨 Run 重写锁和状态转换会改变隔离/冲突边界，当前基线未显示低复杂度替代方案能稳定带来 30% 以上的端到端收益；live lease 拒绝和恢复重放幂等测试仍通过。报告 `data/performance-audit-20261010/expired-run-recovery-before/command.log`。
- 覆盖：本项只记录独立 root Run；child/delegation lineage 已由 PERF-036 单独量化，不能把 root 的批次耗时外推到 child 路径。

### PERF-035：Generation owner JSON 关联索引

- 链路与数据量：Generation owner 查找在取消扫描中按 `resource_preconditions_json->generation_task_id/generation_attempt` 关联 `agent_execution`；夹具写入 **50,000 条**无关 execution，并保留一个有效 owner。
- 根因与改动：原 schema 只有 `(run_id, status, sequence)` 索引，无法支持 JSON 表达式条件。新增幂等 migration `agent_owner_lookup_indexes`，建立 `(generation_task_id expression, generation_attempt expression, run_id)` 索引；未为 Workflow JSON 条件添加未达门槛的索引。
- 量化结果：同一隔离 PostgreSQL 16 schema、2 次预热、15 个样本，正式索引前后 Generation owner lookup P50/P95 **35.696/39.994 → 1.469/1.832 ms**，改善 **95.9%/95.4%**；索引占用 **2,056,192 B**。候选 task 身份保持一致。
- 边界与验证：同一压力夹具的 Workflow 临时索引增量约 14%，拒绝落库。`a4n-db` owner lookup 基准、migration 编译和格式/diff 检查通过。证据 `data/performance-audit-20261010/owner-json-index-final-run/command.log`。

### PERF-036：child/delegation expired-run recovery 基线

- 链路与规模：过期 child Run 恢复除 root 路径的状态 fencing、执行/消息终态和审计事件外，还必须读取并锁定 parent Run、parent/child Thread 与 delegation，更新 delegation 并写入父任务恢复通知。Agent worker 每 2 秒最多处理 16 个过期 lease。
- 量化结果：隔离 PostgreSQL 16、16 个真实 parent/child delegation lineage、2 次预热、15 个样本；完整恢复批次 P50/P95 **194.970/533.421 ms**，16/16 child Run 进入 `recovery_required`。相对 PERF-034 root 批次 111.002/263.408 ms，额外成本来自 parent/delegation/inbox 原子写入。
- 决定：本轮不做跨 child Run 的批量事务重构。它会改变 parent/delegation 锁顺序、并发 fencing 和通知幂等边界，当前没有低复杂度方案能证明稳定达到 30% 收益。证据 `data/performance-audit-20261010/expired-child-recovery-before/command.log`；基准夹具未改产品恢复逻辑。

### PERF-037：独立漫画项目包导出批量读取

- 调用链与规模：用户触发 `export_project_bundle_data` 后，导出应用读取独立漫画包。读取频率由用户操作决定；单次成本随文本资产和冻结来源章节数增长。基准使用 PostgreSQL 16、32 个文本资产（每个 2 个版本）和 128 个来源章节。
- 根因与改动：旧实现每个资产重复校验项目、读取资产及其全部版本；每个来源章节再读章节、来源快照和快照章节。版本读取改为校验项目及资产归属后按 1,000 个资产 ID 分批查询；来源实体按三类 ID 分批查询，再按原来源 ID 顺序组装，缺失和跨项目错误保持显性。
- 量化结果：由查询路径推导 SQL 数 **484 → 10**；2 次预热、15 次交错完整导出，最终复测 P50 **51.346 → 5.365 ms（89.6%）**，P95 **51.885 → 6.007 ms（88.4%）**。每轮完整序列化的导出包逐值相同。
- 语义验证：夹具检查缺失资产仍返回 `NotFound`、跨项目资产仍返回 `Forbidden`；完整包包含所有章节、快照和两个资产版本，源顺序不变。
- 相邻迁移修正：夹具执行 `canvas_project_split::ensure_complete` 时发现预检 SQL 的 Rust 字符串续行把 `m` 与 `JOIN` 拼成 `mJOIN`。改用等价原始多行 SQL；预检和 `ensure_complete` 已由同一真实 PostgreSQL 基准路径覆盖。该修正不计为性能收益。
- 证据：最终运行 `data/performance-audit-20261010/project-bundle-batch/command-05.log` 与 `build-05.log`。前两次初始化失败没有计入性能结果。命令：`cargo test --locked --release -p a4n-db --test project_bundle_batch -- --ignored --nocapture --test-threads=1`，需设置独立的 `A4N_TEST_DATABASE_URL`；性能比较在 release 优化级运行，关闭 LTO 以缩短冷构建。

### PERF-038：项目包历史消息批量导入

- 调用链与规模：项目包导入完成实体/媒体映射后，`import_project_bundle_history` 在一个事务内导入会话及其消息，再导入工作流和生成历史。此处测量对话历史阶段，输入为 1 个会话、1,000 条约 280 B 消息；导入由用户发起。
- 根因与改动：会话先插入以建立源会话到新会话 ID 的映射，之后旧实现逐条 INSERT 消息。现在仍先插入会话并建立同一映射，将消息 ActiveModel 按 1,000 条分批后通过单次 `insert_many` 插入，仍处于原事务中。
- 量化结果：1,000 条消息时由查询路径推导 SQL 数 **1003 → 4**；2 次预热、15 次交错完整历史导入调用。同时间戳排序复测 P50 **86.847 → 14.987 ms（82.7%）**，P95 **98.599 → 18.369 ms（81.4%）**。结果只代表数据库历史导入阶段，不外推到包解析、媒体复制或整体导入响应。
- 语义验证：单条插入基线与正式导入结果计数相同；每两条相邻消息共享时间戳，逐项比较角色、内容、工具调用、token 数、章节号、重要标记和创建时间，含同时间戳的 ID 顺序一致。所有写入仍在单事务中。
- 证据：同库完整双基准见 `data/performance-audit-20261010/project-bundle-batch/command-05.log`；独立同时间戳顺序复测见 `command-06.log` 和 `build-06.log`。数据库名 `a4n_perf_20261010_bundle_arch`，PostgreSQL 16。

### PERF-039：项目包 workflow progress 批量导入

- 调用链与规模：项目包历史导入映射出新 workflow ID 后，导入该 workflow 的 progress 记录。生产路径每个 workflow 最多保留 **200 条**事件；性能夹具使用 1 个 workflow、200 条 progress event。
- 根因与改动：每条事件不需要新的数据库 ID 参与后续映射，但原实现逐条 INSERT。先对所有事件完成 workflow ID 映射、`task_id = None` 和 meta JSON 重映射，再按 1,000 行一批 `insert_many`，事务和失败传播不变。
- 量化结果：查询数由 **203 → 4**；2 次预热、15 次交错完整历史导入，P50 **29.598 → 6.707 ms（77.3%）**，P95 **65.253 → 13.312 ms（79.6%）**。
- 语义验证：event_index、current、total、message、percent、meta、created_at 按顺序逐项一致；来源 task ID 仍在导入时清除。事件源使用两条共用时间戳，排序依靠原 event_index。
- 证据：`data/performance-audit-20261010/project-bundle-batch/command-07.log` 与 `build-07.log`；使用前文相同隔离 PostgreSQL 数据库和定向 ignored benchmark 命令。

### PERF-040：项目包模型用量账本批量导入

- 调用链与规模：项目包导入的 `import_project_bundle_history` 写入工作流历史后，逐条导入关联其 workflow ID 的模型成本账本。基准输入为 1 个 workflow、1,000 条 `model_usage_record`，项目包由用户发起。
- 根因与改动：每条记录都要重生成唯一 request ID、替换 user ID、映射 project/root/workflow/generation ID、清除 provider config ID 并脱敏 pricing snapshot，旧实现每条单独 INSERT。现在仍逐条应用这些转换，再每 1,000 条通过 `insert_many` 写入原事务；无映射/数据库错误仍使整个事务失败。
- 量化结果：SQL **1003 → 4**；2 次预热、15 次交错导入 P50 **192.535 → 42.019 ms（78.2%）**，P95 **305.936 → 52.018 ms（83.0%）**。
- 语义验证：逐行核对生成 request ID 唯一且不同于来源 ID、user/project/root/workflow/generation 映射、provider config 清理、账本金额/状态/时间字段，以及 meter/pricing/request JSON 内容。
- 证据：`data/performance-audit-20261010/project-bundle-batch/command-08.log` 与 `build-08.log`；PostgreSQL 16 独立库，同 PERF-037—039。

### PERF-041：项目包 LLM trace 批量导入

- 调用链与规模：项目包历史导入完成新 workflow ID 映射后，导入模型调用追踪。输入为 1 个 workflow 和 1,000 条 LLM trace，代表约 1,000 次模型调用的历史记录。
- 根因与改动：每条 trace 都独立映射可选的 project/root/workflow/generation ID、脱敏上下文字段并执行 INSERT；这些行不依赖插入结果生成后续 ID。保留逐行映射和脱敏逻辑，将 ActiveModel 按 1,000 条聚合后执行一次 `insert_many`，事务不变。
- 量化结果：真实 PostgreSQL SQL **1003 → 4**；2 次预热、15 次交错导入，P50 **314.611 → 50.811 ms（83.8%）**，P95 **526.321 → 74.528 ms（85.8%）**。
- 语义验证：逐行检查 ID 映射及 call key/fingerprint、workflow/step/provider/model、token 与 cache 用量、上下文 JSON、duration/stream/status/error/finish reason/created_at。
- 证据：`data/performance-audit-20261010/project-bundle-batch/command-09.log` 与 `build-09.log`；PostgreSQL 16 独立库，同 PERF-037—040。

### PERF-042：项目包 generation target 批量导入

- 调用链与规模：项目包历史导入先插入 generation run 建立旧/新 run 映射，再导入该 run 的生成目标。基准为 1 个 workflow、1 个 generation run、1,000 个 target 引用。
- 根因与改动：target 不依赖新 target ID 返回给调用方，但旧实现每行独立执行 generation ID 映射、目标实体 ID 映射/归档和 INSERT。保留这些转换，在同一事务内每 1,000 行批量写入。
- 量化结果：真实 PostgreSQL SQL **1004 → 5**；2 次预热、15 次交错导入，P50 **88.124 → 14.407 ms（83.7%）**，P95 **115.431 → 22.759 ms（80.3%）**。
- 语义验证：target kind/table/映射后 ID/role/created_at 逐项一致，保持原输入顺序；夹具使用专用 `chapter_ids` map 验证 canonical chapter 映射。
- 证据：`data/performance-audit-20261010/project-bundle-batch/command-11.log` 与 `build-11.log`。夹具首次用错通用 entity map，失败发生在计时前，修正为专用 chapter map 后通过。

workflow checkpoint 导入已用 128 条、3.22 MB JSON 做代表性基准；批量写入候选的收益未稳定达到 30% 门槛，故保留逐行转换和插入。每条仍需做 checkpoint contract 校验、source/entity remap、fingerprint 重算和 generation ID 重映射；输入 JSON 体积增长时，后续如有新证据再评估批量化和峰值内存。

### PERF-043：物体档案生成首次事实投影与回读批处理

- 调用链与规模：章节分析的 `object_discovery` 完成模型校验后进入 `upsert_object_profiles_with_generation_run_record`；通常按单章写入一批物体档案，基准覆盖 **16/32/64/128 个**新档案。该路径在同一事务中还要写事实反馈和 text-anchor 证据。
- 根因与改动：每个新 AI 档案的首次事实已经从刚写入的档案生成，旧实现仍查询空历史、重新读取档案并执行等值投影更新；生成结果随后又按档案逐条 `find_by_id`。首次事实现在只保留审计记录，已有投影直接作为当前值；最终档案按 ID 一次批量读取并按原输入顺序组装。已有档案的历史回放、人工字段保护和错误传播保持原路径。
- 量化结果：隔离 PostgreSQL 16、release、2 次预热、15 次样本。最终原始/组合实现的 P50（16/32/64/128）为 **55.350/36.873/45.468/91.485 → 9.991/16.905/29.935/41.031 ms**，下降 **82.0%/54.1%/34.2%/55.1%**；P95 为 **89.445/64.444/159.310/117.310 → 14.008/22.289/34.985/57.138 ms**，下降 **84.3%/65.4%/78.0%/51.3%**。按代码路径，每个全新档案省去历史读取和等值投影的 3 条 SQL，最终回读由 N 条降为 1 条；事务边界没有变化。
- 语义验证：新增成功路径检查确认返回档案与数据库投影字段一致、每个首次事实仍保留 1 条 `material_fact_change`、text-anchor 档案证据仍保留；既有章节来源过期、物体与场景无效证据事务回滚测试通过。小批量单独复测的波动未作为收益结论，以上最终数字来自原始/组合实现各 15 次样本。
- 复现：`A4N_TEST_DATABASE_URL=postgres://.../a4n_perf_20261010_object_generation` 下运行 `cargo test --locked --release -p a4n-db --test object_profile_generation_batch -- --ignored --nocapture --test-threads=1`；基准夹具和成功路径回归位于 `crates/platform/a4n-db/tests/object_profile_generation_batch.rs`。原始和组合命令输出保存在 `data/performance-audit-20261010/object-profile-generation-{before,after}/command.log`。

### PERF-044：Agent thread history 批量预算投影

- 调用链与频率：用户打开 Agent thread history 时，`thread_history` 在同一只读 Repeatable Read 事务中读取最近消息/turn/run/execution/delegation 等数据；基准为单线程 **32 个 Run**，代表历史页一次请求的有界上限。
- 根因与改动：历史数据主查询已批量化，但旧实现随后对每个 Run 调用 `run_budget_snapshot`，重复读取目标 Run、Run 家族和执行事实。改用已有 `run_budget_snapshots` 一次读取所有预算树，并按 Run ID 回填 usage；缺少快照仍显式报错，事务隔离和返回字段不变。
- 量化结果：隔离 PostgreSQL 16、release、15 次交错样本；P50/P95 **8.185/11.335 → 2.766/3.851 ms**，下降 **66.2%/66.0%**。预算相关 SQL 由约 **96 → 3**，消息、turn、execution、事件和预算 usage 逐项一致。
- 证据与复现：`data/performance-audit-20261010/agent-thread-history-before.log`、`agent-thread-history-after.log`；基准及非忽略语义回归位于 `crates/platform/a4n-db/tests/agent_thread_history_batch.rs`。

### PERF-045：故事事件页延迟伏笔索引读取

- 调用链与规模：用户打开小说详情的故事事件页时，页面初载读取章节分析并渲染 **1000 章/5000 个事件**；伏笔索引包含 **1000 条**记录，但 `<details>` 默认收起。此前首屏仍通过 `loadChapters` 请求完整伏笔列表，响应体约 505 KB，且请求与章节渲染竞争浏览器主线程。
- 根因与改动：初载只请求章节分析；用户展开伏笔索引时才调用 `listForeshadowing`。已请求过索引的刷新仍保持原有刷新语义；按项目复用 in-flight Promise，避免展开和刷新同时触发重复读取。项目切换、迟到错误和 loading 状态均按当前项目门控，完整记录、错误提示、编辑/删除和键盘展开行为不变。
- 量化结果：真实 Chromium **153.0.8010.52**、相同生产构建、1000 章/5000 事件/1000 条伏笔、章节接口 25 ms、伏笔接口 150 ms 的受控三轮 A/B 中位数，首屏可用时间 **342.7 → 196.2 ms（下降 42.7%）**；初始伏笔 JSON **505,677 → 0 B**。新版展开后的完整列表读取和渲染中位数约 245.7 ms，属于用户主动打开索引后的延迟，未隐藏或截断数据。
- 语义验证：每轮均确认展开后 1000 条记录、Enter 收起/重新展开后仍为 1000 条；初始请求数只包含章节分析，展开后恰好一次 `list_foreshadowing`。生产构建、API parity、现有伏笔索引 Vitest 2 项和 `git diff --check` 通过。夹具只证明首屏请求路径与渲染行为，不代替真实数据库端到端延迟。
- 证据与复现：`data/performance-audit-20261010/perf045/{before-1-retry,before-2,before-3,after-1,after-2,after-3}/report.json`；从 `frontend` 执行 `env -u http_proxy -u https_proxy -u all_proxy -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u NODE_USE_ENV_PROXY A4N_PERF_CASE=story-events-loader A4N_PERF_BASELINE_REF=HEAD A4N_PERF_ARTIFACTS=<独立目录> A4N_PERF_PORT=<空闲端口> node e2e/performance/check.mjs`，新版去掉 `A4N_PERF_BASELINE_REF`。

### PERF-046：Agent 消息列表无关状态重渲染

- 调用链与频率：Agent Monitor 和领域 Agent 控制台在输入草稿、展开活动面板、刷新 loading 或事件连接状态变化时会重渲染消息区域；真实历史可达数百到上千条消息，Agent 流式更新还会持续触发状态更新。旧 `AgentMessageList` 每次都重新分组全部消息并协调全部消息行。
- 根因与改动：消息列表只依赖 `messages`、`locale`、流式消息 ID、待发送消息和当前运行状态。组件改为 React `memo`，比较这些语义输入；空的待发送数组按长度处理，父级每次创建的新空数组不会误触发。任一消息数组、流式集合、待发送消息或运行状态变化仍会完整渲染，工具调用配对和消息顺序未改变。
- 量化结果：真实 Chromium **153.0.8010.52**、生产构建、1000 条交替用户/助手消息、80 次无关状态更新的三轮 A/B，中位数 **149.6 → 0.1 ms（下降 99.9%）**；两侧均保留 1000 个消息行，首尾 HTML SHA-256 均为 `9b86859086ff3fe964b663ae17f28aa79353c9035a6979dbbbc291b09de29ea7`。该基准测量的是高频前端重渲染 CPU，不把消息 API 或模型响应延迟写成收益。
- 语义验证：`AgentMessageRow.test.tsx` 12 项通过；基准每轮检查 1000 个 `[data-agent-message-role]` 行和 DOM 摘要，memo 只跳过无关父状态更新，不减少数据或改变工具结果配对。
- 证据与复现：`data/performance-audit-20261010/agent-messages/{before-1,before-2,before-3,after-1,after-2,after-3,before-4,after-4}/report.json`；从 `frontend` 执行 `env -u http_proxy -u https_proxy -u all_proxy -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u NODE_USE_ENV_PROXY A4N_PERF_CASE=agent-messages A4N_PERF_BASELINE_REF=HEAD A4N_PERF_ARTIFACTS=<独立目录> A4N_PERF_PORT=<空闲端口> node e2e/performance/check.mjs`，新版去掉 `A4N_PERF_BASELINE_REF`。

### PERF-047：档案素材图片计数聚合

- 调用链与规模：对象/场景档案 worker 列表请求需要为每个档案显示已生成素材图数量；档案删除还需要读取所有下游图片 ID。旧路径按项目和图片类型读取全部 `id/image_type/metadata_json`，再在 Rust 中解析每条图片。基准为 **20,000 张**对象素材图片，每条元数据约 **1.75 KB**，100 个档案。
- 根因与改动：计数只需要 `profile_kind/profile_id` 和行数，删除只需要匹配 identity 后的 ID；完整 JSON 回传与逐条反序列化属于请求链路中的无效开销。计数改为 PostgreSQL 按 JSON profile identity 聚合，删除改为 PostgreSQL 按同一 identity 直接筛选 ID；Rust 仍解析 JSON 数值并保留正整数及请求 ID过滤。未知图片类型、错误 profile kind、字符串/负数/小数/null ID 仍不计入。
- 量化结果：隔离 PostgreSQL 16、9 次交错样本。计数旧/新 P50/P95 **277.905/307.716 → 39.846/44.813 ms**，下降 **85.7%/85.4%**；删除 ID 旧/新 P50/P95 **275.986/283.187 → 27.412/36.981 ms**，下降 **90.1%/86.9%**。20,000 张图片的计数逐档案相同，删除 ID 按原 ID 顺序相同。
- 语义验证：夹具包含字符串、错误 kind、负数、小数和 null profile ID，旧/新计数与 ID 结果逐项相等；`a4n-db` profile_material 单测 **7/7** 通过。SQL 使用绑定参数，无用户输入拼接；完整素材列表读取路径没有改变。
- 证据与复现：`data/performance-audit-20261010/profile-material-image-counts/command.log`；从工作树执行 `flock -x /home/fufu/Code/A4N/work/.agent-build.lock env A4N_TEST_DATABASE_URL=postgres://.../a4n_test CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 cargo test --locked -p a4n-db --test profile_material_image_counts_batch benchmark_profile_material_image_counts -- --ignored --nocapture --test-threads=1`。

### PERF-049：依赖图族级图片窄投影

- 调用链与规模：Canvas 资产图 worker 的 `get_project_asset_dependency_graph` 先加载项目族记录，再由图 builder 只计算漫画页、对象/场景素材和视频帧图片数量；节点详情接口不复用这批摘要记录。基准使用 PostgreSQL 16、**12,000 张** `generated_image`，其中 **600 张**属于图摘要消费的六种类型，其余图片携带约 3.5 KiB 无关 JSON。
- 根因与改动：旧查询按项目族读取每张图片的 `image_type/metadata_json`，随后在 Rust 中丢弃无关类型并解析剩余元数据。族级摘要查询增加 `image_type IN (...)` 过滤，只把计数所需的六种类型交给现有 builder；没有改变节点详情查询，也没有新增缓存或预取。
- 量化结果：隔离 PostgreSQL 16，9 次交错样本，优化构建。窄投影返回行 **12,000 → 600**，元数据字节 **42,942,240 → 32,640**；查询 P50/P95 **56.783/65.582 → 3.324/5.278 ms**，下降 **94.1%/92.0%**，超过保留门槛。
- 语义验证：过滤前结果按六种类型筛选后与新查询逐行相等；完整依赖图的漫画图片计数保持 100，视频帧和素材类型测试通过；节点详情仍使用原始完整图片字段和排序。SQL 使用 SeaORM 绑定参数，用户项目范围仍由既有 scope 查询决定。
- 证据与复现：`data/performance-audit-20261010/dependency-graph-image-projection/command.log`；从工作树执行 `flock -x /home/fufu/Code/A4N/work/.agent-build.lock env A4N_TEST_DATABASE_URL=postgres://.../a4n_perf_arch_20261011 CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 cargo test --locked --release -p a4n-db --test dependency_graph_batch benchmark_dependency_graph_image_projection -- --ignored --nocapture --test-threads=1`。

### PERF-050：任务历史窄投影

- 调用链与规模：设置页/任务历史经 `get_task_history`、`TaskReadApplicationPort` 进入 `persisted_task_history`；列表读取由用户操作触发，分页参数由既有 Runtime 校验。基准使用 PostgreSQL 16 中 **2,000 条任务**、每条约 **32.5 KB payload**，对比 **200 条**历史页。
- 根因与改动：摘要转换实际只消费 ID、任务类别/优先级/状态、项目归属、workflow ID、进度、错误和时间字段；旧 ORM `all` 同时读取并反序列化 `payload`、`dag_id`。列表和按 ID 读取现在共用显式列投影；两种行类型转成同一组内部字段，再走原来的 task kind/status/progress 校验和错误映射。
- 量化结果：隔离 PostgreSQL 16，9 次交错样本，release 构建。200 行旧路径读取 **6,505,200 B payload**；新查询不读取该列，Rust 投影记录序列化为 **82,990 B**。查询 P50/P95 **7.266/13.174 → 2.096/2.677 ms**，分别下降 **71.1%/79.7%**，超过门槛。
- 语义验证：新旧 200 条记录按时间顺序逐行比较 ID、任务类别、优先级、状态、失败/中断原因、workflow/project/source scope、进度 JSON 和创建/开始/完成时间；按 ID 读取与列表首行序列化一致。`workflow::tests::task_info_from_row` 2/2 通过，保留 failed 缺失错误时显式失败的既有行为。数据库队列行及取消/恢复流程未修改。
- 证据与复现：`data/performance-audit-20261010/task-history-projection/benchmark.log`；从工作树执行 `flock -x /home/fufu/Code/A4N/work/.agent-build.lock env A4N_TEST_DATABASE_URL=postgres://.../a4n_perf_task_hist_arch_20261011 CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 cargo test --locked --release -p a4n-db --test task_history_projection benchmark_task_history_projection -- --ignored --nocapture --test-threads=1`。

### PERF-051：轮询工作流详情省略 generation payload

- 调用链与频率：Canvas `loadWorkflowStatus` 在 workflow 为 `running/pending` 时每秒请求 `get_workflow_run_detail`，设置页在展开 workflow 时读取详情。两处界面只呈现 generation run 标题、状态、次数及 raw/parsed/validation 是否存在；CLI 工作流详情仍需要完整诊断 JSON。
- 根因与改动：旧 DB 投影对最多 **300 条** generation run 都读取并反序列化 input fingerprint、raw response、parsed output、validation 和 artifact refs JSON。新增可选 `include_generation_run_payloads`：未提供时维持旧完整结果；UI 显式设为 false 时仅返回既有摘要字段和三个存在标记。read worker contract 升至 v3，避免旧 worker 把新增输入字段当成未知字段；未传该字段的旧 CLI 请求仍走完整路径。
- 量化结果：隔离 PostgreSQL 16、**300 条** generation run，每条 fingerprint/raw/parsed JSON 各含约 19 KB 文本，9 轮交错 release 测量。真实 `workflow_run_detail` 完整结果与轻量结果序列化大小 **17,324,230 → 195,978 B（下降 98.9%）**；DB 详情 P50/P95 **18.561/26.785 → 2.528/3.145 ms（下降 86.4%/88.3%）**。
- 语义验证：新旧 300 条 run 的 ID、工作流/步骤、调用 key、标题、模型、状态、attempt、响应摘要、错误和时间逐项相同；raw/parsed/validation/artifact/fingerprint 大对象仅轻量模式省略，三个 UI 使用的内容存在标记逐项相同。DB 详情测试验证完整模式保留 JSON、轻量模式只省略 JSON；前端诊断改读存在标记。CLI 未设置新字段时默认为完整模式。
- 证据与复现：`data/performance-audit-20261010/workflow-generation-detail-projection/benchmark.log`；从工作树执行 `flock -x /home/fufu/Code/A4N/work/.agent-build.lock env A4N_TEST_DATABASE_URL=postgres://.../a4n_perf_workflow_detail_arch_20261011 CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 cargo test --locked --release -p a4n-db --test workflow_generation_detail_projection benchmark_workflow_generation_detail_projection -- --ignored --nocapture --test-threads=1`。

### PERF-052：测试中心日志增量投影

- 调用链与频率：Test Center 对 `queued/running` run 每 **800 ms** 调用 `get_test_run`。`TestConsole::get` 命中活动 run 的内存记录，但旧实现仍克隆完整 `TestRunDetail`，每次返回最多 **500 条**日志；`append_log` 将每条日志限制为 **2,000 字符**。
- 根因与改动：页面已掌握上次收到的日志数量，却反复下载和克隆此前完整日志。`get_test_run` 增加可选 `after_log_index`，响应报告实际保留窗口的 `log_start_index`；活动缓存命中时仅克隆摘要、参数、步骤和游标后的日志。前端用游标合并增量，服务端移除旧日志时以前端可检测的起点重置窗口。旧请求省略游标仍返回完整日志，旧持久报告缺少起点时默认 0；过期游标返回完整保留窗口，超前游标明确失败。
- 量化结果：release 测试夹具使用 **500 条 × 2,000 个 ASCII 字符**，计时包含生产 `TestConsole::get` 和 `serde_json` 响应序列化。完整响应 **1,001,914 B**，游标指向末尾时增量响应 **415 B**；完整/增量 P50/P95 **0.676/0.947 → 0.001/0.002 ms**，P50 下降 **99.8%**。该基准测量 Runtime 读取与序列化，不外推 HTTP/网络时延。
- 语义验证：Runtime 测试 **11/11** 覆盖完整读取、增量读取、过期/超前游标、500 条轮转、旧报告默认值及既有恢复/删除行为；API 合同测试旧请求省略游标、前端 API 测试请求形状和日志窗口合并测试均通过。未轮询的旧调用方仍读取全量日志。
- 证据与复现：`data/performance-audit-20261010/test-run-log-projection/benchmark.log`；从工作树执行 `flock -x /home/fufu/Code/A4N/work/.agent-build.lock env CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 cargo test --locked --release -p a4n-runtime benchmark_test_run_detail_log_projection --lib -- --ignored --nocapture --test-threads=1`。

### PERF-053：任务进度事件增量轮询

- 调用链与频率：通用 Host progress task 每 **500 ms** 拉取最多 **200** 个事件；Canvas workflow 状态每秒同时拉取最多 **20** 个事件。通用任务事件位于有界 200 项内存 ring，workflow 事件持久化在 PostgreSQL；两处前端都以事件 ID 去重/展示，Canvas 只呈现最后五条。stream text 已独立限制为 **512 字符尾部**，不纳入事件 cursor 改动。
- 根因与改动：旧请求每次都克隆、反序列化/投影并传输最近完整事件页，前端再丢掉已见的 ID。增加可选公开 `after_event_id`，task ring 按 ID 返回后续窗口，workflow read worker 按持久事件 ID 过滤；前端每次成功后推进 cursor。task cursor 在 200 项 ring 内找不到时回退当前最近窗口；workflow 按 cursor 后续 ID 读取。worker 输入增加数值 cursor，合同版本升为 `workflow-read-v4`。旧客户端不传游标时仍取完整最近页，stream text 照旧每次返回。
- 量化结果：release task fixture 使用 **200 条 × 256 ASCII 字符**和 512 字符 stream snapshot，序列化 response **71,031 → 894 B**，读事件加序列化 P50/P95 **0.332/0.420 → 0.004/0.006 ms（P50 下降 98.7%）**。隔离 PostgreSQL 16 workflow fixture 使用 **200 条 × 256 ASCII 字符**，Rust debug 测试 profile 数据库读的 P50/P95 **3.068/5.256 → 0.639/1.016 ms（下降 79.2%/80.7%）**，序列化 response **73,213 → 398 B**。两组新查询均只返回 cursor 后剩余的 1 条事件。
- 语义验证：task ring cursor 命中、过期 cursor 回退、数据库 cursor 顺序与既有 progress snapshot/cancellation 回归通过；API 合同旧请求缺省和 command manifest 自检通过；前端定向 Vitest 4/4 检查首次全页和后续 cursor 参数。`workflow-read-worker` request contract/caller 与全部受影响 crate tests 编译通过，worker v4 catalog 一致。`a4n-task-runtime` 全 crate 串行测试有 35 项通过、3 项因既有 image/LLM/video 配置 fixture 缺少现为 NOT NULL 的 `pricing_json`/`agent_protocol` 失败，另 1 项 benchmark 忽略；内存 task ring 定向测试通过，这些配置 fixture 与 progress cursor 无关。
- 证据与复现：`data/performance-audit-20261010/progress-event-cursor/benchmark.log`。task 基准命令 `cargo test --locked --release -p a4n-task-runtime benchmark_progress_event_cursor_projection --lib -- --ignored --nocapture --test-threads=1`；PostgreSQL 基准在隔离库 `a4n_perf_progress_cursor_20261011` 中运行 `cargo test --locked -p a4n-db --test workflow_batch_queries benchmark_workflow_progress_event_cursor_projection -- --ignored --nocapture --test-threads=1`。初次测试 runner 拉取 `postgres:16` 因 Docker Hub 超时失败；改在本机现存 PostgreSQL 16 容器里新建独立 benchmark 数据库后完成同一 fixture，不使用 debug 数据库。

### PERF-054：测试中心历史列表终态报告窄投影

- 调用链与频率：Test Center 页面初始化和测试结束刷新调用 `list_test_runs(30)`。旧实现扫描并读取目录内**全部** JSON 报告，再只返回最近 30 条；每份报告最多 **500 条、每条 2,000 字符**日志，而列表只返回 `TestRunSummary`。
- 根因与改动：终态报告不需要 parameters、steps 或日志正文。新增 typed summary projection，仍解析参数/步骤结构、日志字符串类型和游标字段以保留损坏报告错误；只有 queued/running（需要 Host 重启恢复）才再次完整解析，缓存优先级、活动运行、排序、重复文件和旧报告默认值保持原行为。
- 量化结果：Linux 独立子进程、**40** 份报告、每份 **500×2,000** 字符日志、返回 30 条，release 交错测量；基线复刻旧完整 `list()` 路径（包括缓存/活动状态检查），摘要 JSON 字节完全一致。P50/P95 **17.778/37.523 → 11.510/14.530 ms**（下降 **35.3%/61.3%**），峰值 RSS **51,268 → 13,004 KiB**（下降 **74.6%**），同时达到时延和内存门槛。
- 语义验证：projection 与完整解析摘要及响应字节逐项一致；错误日志类型与旧解析路径返回相同错误文本；stale queued 报告仍恢复为 failed 并持久化，活动 running 报告不恢复。Runtime 列表/恢复/结构模块测试 **14 passed、2 ignored**；`cargo clippy --offline --locked --all-targets -p a4n-runtime` 通过，输出仅有既有代码告警。
- 证据与复现：`data/performance-audit-20261010/test-run-list-projection/benchmark.log`；`flock -x /home/fufu/Code/A4N/work/.agent-build.lock env CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 cargo test --locked --release -p a4n-runtime benchmark_test_run_summary_list_projection --lib -- --ignored --nocapture --test-threads=1`。

### PERF-055：章节生成人物参考上下文批处理

- 调用链与频率：每次章节生成准备阶段构造人物参考上下文；测试覆盖 **16/32/64 个**已发现人物档案，每个档案均绑定当前首见章节锚点及完成的 `character_discovery` run。
- 根因与改动：旧路径逐档案校验文本根项目和身份、查询首见证据/锚点/章节正文、读取时间线事件/删除标记/discovery run，并读取可回滚的语音/弧线事实。现在文本证据、时间线所有权与回放、profile enrichment 在各自 owner 中批量读取；时间线档案按 **128** 项分批，原状态回放与事实撤销规则保持不变，不增加业务缓存或预取。
- 量化结果：同一隔离 PostgreSQL 16 库和同一 fixtures 下交错执行旧逐档案基线与完整新 `chapter_generation_reference_context_rows`，15 个样本、2 次预热，包含结果序列化且逐次响应字节相同。P50/P95：16 档 **62.964/75.181 → 7.428/9.614 ms**（88.2%/87.2%）；32 档 **127.599/141.437 → 8.199/10.110 ms**（93.6%/92.9%）；64 档 **247.041/290.149 → 7.523/10.703 ms**（97.0%/96.3%）。这是数据库 context read/projection，不包含 LLM 上游生成时延。
- 语义验证：新旧人物记录序列化字节逐项相同；集成回归 **1/1** 覆盖 future profile 排除、AI 时间线与用户删除标记优先级、未来章节语音事实撤销；ignored A/B 基准 **1/1**。`cargo clippy --offline --locked --all-targets -p a4n-db` 通过，输出仅有既有 lint 告警。工作集分批限制为最多 128 份人物时间线记录。
- 证据与复现：旧路径基线 `data/performance-audit-20261010/chapter-generation-reference-context/baseline.log`；同进程交错 A/B `data/performance-audit-20261010/chapter-generation-reference-context/after-candidate.log`。在隔离库 `a4n_perf_ctx_arch` 中运行 `cargo test --locked --release -p a4n-db --test chapter_generation_reference_context_batch -- --ignored --nocapture --test-threads=1`。

### PERF-056：事实反馈摘要仅读取最新有效变更

- 调用链与频率：Fact Feedback 面板在归纳工作流运行时每 **1.5 秒**轮询 `get_project_fact_feedback`；`material_fact_change` 越积越多，但面板仅显示 ID 最新的 **100** 条非章节反馈且 `before != after` 的变更。旧路径读取项目全部非章节变更及大 JSON，再由 Orchestrator 过滤、截断。
- 根因与改动：查询按 `id DESC` 以 **128** 条分页，数据库使用 `(project_id, id DESC) WHERE material_kind <> 'chapter_feedback'` 部分索引；Rust 按原 `serde_json::Value` 相等语义过滤并在凑齐 100 条后停止。保留原始记录顺序和摘要序列化。没有在 SQL 使用 JSONB 不等式：PostgreSQL 会认为 `1` 与 `1.0` 相等，但 `serde_json::Value` 会区分两者；新旧对照夹具特意把这类记录放在最新位置，确保它被包含。
- 量化结果：隔离 PostgreSQL 16 release，目标项目 **5,000** 条大变更（每行 before/after/evidence 各约 10.5 KiB，共约 32 KiB），另加入 **50,000** 条其他项目记录模拟全库索引噪声。旧路径物化 **4,545** 个目标项目模型，新路径从部分索引读取 **128** 条以取得 100 条摘要。15 次交错样本、2 次预热，数据库行序列化结果字节相同；P50/P95 **386.297/391.092 → 15.198/16.787 ms**，下降 **96.1%/95.7%**。按行数估算目标路径模型物化量约下降 **97.2%**，未把估算写成实测 RSS。
- 语义验证：返回的 100 项序列化字节逐项一致，包含最新 `1`/`1.0` 差异、相等 before/after 过滤、`chapter_feedback` 排除和 ID 降序；release A/B **1/1** 通过。`EXPLAIN` 确认分页查询使用 `idx_material_fact_change_summary_latest`，按 `project_id` 索引扫描；`a4n-db` 与 `a4n-migration` 编译、全仓格式检查通过。
- 证据与复现：`data/performance-audit-20261010/fact-feedback-read-projection/benchmark.log`；隔离库 `a4n_perf_fact_feedback_20261011` 中执行 `cargo test --locked --release -p a4n-db --test fact_feedback_read_projection benchmark_fact_feedback_summary_projection -- --ignored --nocapture --test-threads=1`。索引迁移为 `fact_feedback_read_indexes`。

### PERF-057：章纲列表窄投影

- 调用链与频率：写作台和小说大纲页在项目初载、手动/写后刷新时读取章纲列表；页面渲染、编辑和章节生成上下文只使用 ID、章节号、标题、摘要、目标字数、来源类型及更新时间。大项目按 **1000 章**测量；不是后台轮询路径。
- 根因与改动：旧 `list_chapter_outlines` 每次读取、映射并序列化节拍、关键场景、伏笔、角色和状态等完整 JSON；两个列表并不消费这些字段。新增 `list_chapter_outline_index`，数据库仅投影页面字段，来源读取仍只取 `outline_id/source_kind`；两个页面切换到窄读口，原完整命令保留给仍需要完整章纲的调用方。项目权限、章节排序、来源标记、保存/生成后的刷新和错误路径沿用原边界。
- 量化结果：隔离 PostgreSQL 16，1000 条章纲，完整 JSON 响应平均约 **27 KB/章**；同一 release 测试进程交错读取旧完整模型与新投影，包含读取、映射和 JSON 序列化。完整/窄响应 **27,335,573 → 198,573 B（减少 99.27%）**；P50/P95 **72.082/78.865 → 3.057/3.587 ms（减少 95.76%/95.45%）**。这是 DB 读取与响应序列化测量，不外推网络端到端时延。
- 语义验证：1000 条旧列表投影与新列表项逐项相同；真实 Chromium 153 检查生产 `ChapterOutlineSection` 的旧/新字段，1440/390px、`zh-CN`/`en-US` 的 HTML 与文本哈希一致，Enter 打开相同章节且无横向溢出。前端 API 测试 **6/6**、Host command manifest/API contract **110/110**、outline worker **2/2**、worker contract **1/1**、process catalog **10/10**、类型生成器 **5/5**、API parity、前端生产构建、受影响 crates all-target Clippy、rustfmt 和 diff 检查通过；Clippy 输出为依赖/既有代码告警，本项新增代码无告警。
- Mock Provider E2E：按 `scripts/tests/CREATION_E2E.md` 实际运行大纲阶段。Chromium 创作流程中，新 `/api/commands/list_chapter_outline_index` 读口被调用两次且均返回 HTTP 200；后续“生成分部大纲”失败，浏览器错误为提示词装配模块 `primary` 缺少 `optimization_direction`。CLI authoring 也在 outline task 失败；阶段未完成，完整 E2E 不能计为通过。报告为 `data/performance-audit-20261010/outline-list-projection/creation-e2e-outline-rerun/report.json`。性能候选不修改提示词配置。
- 证据与复现：PostgreSQL A/B 日志 `data/performance-audit-20261010/outline-list-projection/benchmark.log`，Chromium 报告 `data/performance-audit-20261010/outline-list-projection/browser-final-recheck/report.json`，Rust suites 日志 `data/performance-audit-20261010/outline-list-projection/unit-test.log`。在隔离数据库中运行 `flock -x /home/fufu/Code/A4N/work/.agent-build.lock env CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 A4N_TEST_DATABASE_URL=<isolated-postgres-url> timeout 60s cargo test --locked --release -p a4n-db --test chapter_outline_list_projection benchmark_chapter_outline_list_projection -- --ignored --nocapture --test-threads=1`。

### PERF-058：故事事件列表分析数据投影

- 调用链与频率：Novel Detail 的 Story Events 页在项目载入和手动刷新时读取每章分析；这不是轮询路径。千章项目列表包含 **1000 章 / 900 条分析 / 4500 个事件**。组件只显示 `metadata.characters/locations/tone/timeline_marker`、`summaries.compressed/summary/short/one_line/keywords`、`key_events`，以及 planted/resolved/tensions 三个伏笔计数；独立伏笔索引继续按原来的展开流程读取。
- 根因与改动：旧 `list_chapter_analyses` 将整份 `analysis_data` 发给只消费以上字段的页面。新增 `list_story_event_chapters`，在拥有 JSON 的 domain 层原地删去未展示字段，并以计数代替完整伏笔数组；沿用相同章节查询、顺序、项目 Viewer 权限与错误路径。Host、Worker（contract v6）、前端 API 均接入新读口，旧完整合同保留。故事事件提取函数直接读取计数字段，不重建已经丢弃的数组。
- 量化结果：release PostgreSQL A/B 的产品 in-place 投影将 JSON 响应 **5,618,404 → 2,285,686 B**；读取、映射和序列化 P50/P95 **27.225/33.622 → 25.293/30.445 ms**（下降 **7.1%/9.4%**，未达到响应时延门槛）。真实 Chromium 153、三轮独立进程中位数下，GC 后页面 JS heap：**1440px 26,712,948 → 17,919,559 B（下降 32.92%）**，**390px 26,716,284 → 17,910,184 B（下降 32.96%）**；达到门槛的收益为保留 JS 堆内存下降。传输夹具 raw/gzip JSON **5,584,005/92,374 → 2,251,287/40,772 B**（分别减少 **59.68%/55.86%**）。
- 语义验证：六对浏览器 A/B 的 HTML 与文本 SHA-256 完全相同，章节/分析数、事件行与 DOM 数相同；搜索 `999` 仍显示一章五事件，Enter 仍打开第 999 章，桌面和手机横向溢出均为 0。CLI mock-provider E2E 通过：隔离项目导入一章后，旧/新列表的共有字段逐项相同，新读口返回 `{planted: 0, resolved: 0, tensions: 0}`；`run_creation_e2e.py --stage inspiration` 的 CLI 与浏览器阶段均通过。React commit 中位数桌面 **183.9 → 175.2 ms**、手机 **182.7 → 177.4 ms**，降幅低于本项目门槛；API wrapper 和 domain 投影测试通过。
- 边界与复现：SQL 内重建 JSON 的候选 P50/P95 为 **159.380/161.917 ms**，已拒绝。本轮没有重跑 release DB microbenchmark，采用现有隔离 PostgreSQL 16 记录 `data/performance-audit-20261010/story-event-analysis-projection/benchmark.log`。真实 CLI/Host/Worker E2E 报告：`data/performance-audit-20261010/story-event-analysis-projection/cli-e2e-20261012/report.json`；复现命令为 `python3 scripts/tests/run_creation_e2e.py --stage inspiration --postgres-image docker.m.daocloud.io/library/postgres:16 --report-dir data/performance-audit-20261010/story-event-analysis-projection/cli-e2e-20261012`。Chromium 原始报告位于 `data/performance-audit-20261010/story-event-analysis-projection/final-direct/{full,projected}-{1440,390}-{1,2,3}/report.json`。从 `frontend` 重放：基线侧设置 `A4N_PERF_BASELINE_REF=4d52b9eba` 和 `A4N_PERF_PROJECTION_VARIANT=full`；投影侧省略 baseline ref 并设置 variant 为 `projected`。`A4N_PERF_VIEWPORT` 分别设 `1440`、`390`，每个样本使用独立的 `A4N_PERF_ARTIFACTS` 路径，交错执行三轮 `env -u http_proxy -u https_proxy -u all_proxy -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u NODE_USE_ENV_PROXY A4N_PERF_CASE=story-events-analysis-projection ... node e2e/performance/check.mjs`。
- 验证：受影响 crates `cargo check --all-targets` 通过；domain 投影 **2/2**、API command manifest **46/46**、process catalog **10/10**；前端定向 Vitest **11/11**、API parity、生产构建、CLI/Host/Worker mock-provider E2E、rustfmt 与 diff 检查通过。生产构建仍显示既有 1 MB 主 chunk 警告。

### 拒绝候选：项目列表协作成员 SQL 聚合

- 调用链与频率：`NovelList` 初载和筛选时按 20 个项目一页调用 `list_projects`；Project Read Worker 批量构造摘要时调用 `project_access_info_batch`。协作访问层当前读取这些根项目的 `(project_id, user_id, role)` 全部成员行，再在 Rust 中算成员数和当前用户角色。它不是轮询路径。
- 候选：数据库按 `project_id` 聚合 `COUNT(*)`，用 `MAX(role) FILTER (WHERE user_id = $2)` 一并取当前用户角色，再把最多 20 个摘要交给现有权限与项目组装代码。对应 SQL 形状为 `SELECT project_id, COUNT(*)::BIGINT, MAX(role) FILTER (WHERE user_id = $2) FROM project_member WHERE project_id = ANY($1) GROUP BY project_id`；绑定根项目 ID 数组和当前 user_id。`project_member(project_id,user_id)` 唯一索引确保每项目最多一个当前用户成员行；owner / viewer、成员数和排序合同不变。
- 量化结果：隔离 PostgreSQL 16，**20 个项目 × 1000 个成员**，同一 viewer 加载 20 个项目，比较完整 `project_access_info_batch`，2 次预热、15 次样本。原查询 P50/P95 **10.472/15.964 ms**；临时聚合候选 **9.547/12.006 ms**，仅下降 **8.83%/24.79%**，未达到 30% 响应时延门槛。候选虽把 DB 返回元组从 **20,000 → 20**，但没有测得总内存降低 20%，所以未保留生产 SQL 改动。
- 语义验证与决定：两版都返回 **20** 个项目，每个 member_count 为 **1000**，当前角色为 Viewer，owner 字段与 pending-invite 可见性一致。候选实现已撤回；保留当前基线的忽略型 benchmark 与隔离运行日志，避免把大团队的潜在收益当成已达门槛的产品优化。
- 证据：基准夹具 `crates/platform/a4n-db/tests/project_access_member_projection.rs`；`data/performance-audit-20261010/collaboration-access-member-projection/{before,after}/command.log` 与对应 `database.json`。`before` 测当前实现，`after` 为已撤回的聚合候选。baseline 可在 `.agent-build.lock` 独占锁中用 `CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_RELEASE_DEBUG=0 TMPDIR=/home/fufu/Code/A4N/work/.p`，再调用 `scripts/tests/run_with_test_database.py --image docker.m.daocloud.io/library/postgres:16 --report-dir <new-worktree-data-dir> -- cargo test --locked --release -p a4n-db --test project_access_member_projection benchmark_project_access_member_projection -- --ignored --nocapture --test-threads=1` 复测。

### 拒绝候选：工作流列表恢复快照窄投影

- 链路与规模：设置页 QueueTab 每 **5 秒**请求最多 **40** 条 `list_workflow_runs`；工作流行的 `progress_json` 同时保存公开进度和恢复 `snapshot`。候选只在公开列表读口中用 PostgreSQL `json::jsonb` 运算移除 `snapshot` 与取消任务字段，恢复、详情和编排读口保持完整模型。
- 语义验证：夹具包含 **40** 条运行记录，每条恢复快照约 **512 KiB**。候选与旧路径的 `WorkflowRunSummary` 序列化响应逐字节一致，公开进度和排序一致；完整路径显性拒绝非法 `ProgressMeta` 的错误也保留。
- 量化结果：隔离 PostgreSQL 16、release、15 次交错样本、2 次预热。旧路径 P50/P95 **19.208/25.550 ms**，候选 **76.210/78.058 ms**，反而慢 **296.8%/205.5%**；候选虽把进程内 `progress_json` 从 **20,977,990 B** 降到 **3,840 B**，但数据库端 JSON 转换成本主导请求时延，未达到响应门槛且方向相反。
- 决定与证据：撤回所有生产代码改动，不改变 `list_workflow_runs` 合同。保留一次输入夹具失败（非法 `ProgressMeta`）和通过样本的私有 runner 报告：`data/performance-audit-20261010/workflow-run-list-projection/`、`data/performance-audit-20261010/workflow-run-list-projection-rerun/`、`data/performance-audit-20261010/workflow-run-list-projection-rerun2/`。后续若要优化该路径，应先在写入侧拆出可直接索引的公开进度列，再重新测量，不能在查询时重建 JSON。

### PERF-061：导入审核队列索引列表

- 调用链与频率：Novel Detail 的 Imports 页在进入页面和每 **5 秒**刷新一次导入审核队列；列表卡片只显示文件名、审核状态和时间，`analysis_result` 只在打开详情或编辑审核 JSON 时使用。基准为 **100** 条审核项，每条分析 JSON 约 **64 KiB**。
- 根因与改动：旧 `list_import_queue` 每次完整读取并序列化所有 `analysis_result`。保留旧完整命令给兼容调用方，新增 `list_import_queue_index` 的数据库窄投影和 `get_import_review` 详情读口；页面轮询改用索引，点击详情和已打开详情的刷新才按 ID 取完整 JSON。权限、状态解析、排序和旧命令合同不变。
- 量化结果：隔离 PostgreSQL 16、release、15 次交错样本、2 次预热。完整列表 P50/P95 **7.010/9.507 → 0.382/0.535 ms**，下降 **94.4%/94.4%**；响应序列化 **6,580,311 → 16,627 B**，减少 **99.75%**。索引结果逐项保留 ID、项目、文件名、状态和时间字段。
- 验证与证据：基准夹具 `crates/platform/a4n-db/tests/import_review_list_projection.rs`；报告 `data/performance-audit-20261010/import-review-list-projection/command.log`。Rust 受影响 crates `cargo check --locked`、前端 `npm run check:web-api-parity` 和 `npm run build` 通过；旧完整读口未删除，详情错误仍显性返回。

### PERF-062：物体档案工作流列表窄投影

- 调用链与频率：写作台分析页按章节展示用户手工物体证据，Novel Detail 的 Images 页在打开/刷新时为生图选择物体档案；两处都只消费档案索引字段。档案管理页仍依赖完整记录。基准为 **1000** 个档案、每个 **7 个约 2 KiB 文本字段**、**512 B 证据**和 **1000 张**关联物体图片。
- 根因与改动：旧列表读取、克隆并序列化全部档案字段，包括这些列表不使用的别名、材质状态、视觉/生成用途、证据正文和生图提示词。新增数据库窄投影及 `list_object_profile_index` Worker/API 读口，两处索引消费者切换到新读口；旧完整合同保留给档案管理页及兼容调用方。权限校验、图片计数、项目校验、按首次章节及 ID 排序和缺失图片计数为 0 的行为不变。
- 量化结果：隔离 PostgreSQL 16、release、2 次预热、15 次交错样本，计时包括 Viewer 权限查询、旧/新数据库读取、批量图片计数、字段映射和 JSON 序列化。完整/窄列表 P50 **97.502 → 53.404 ms（下降 45.2%）**，P95 **104.937 → 58.155 ms（下降 44.6%）**；序列化响应 **15,356,224 → 6,507,334 B（减少 57.6%）**。
- 语义验证与复现：1000 项顺序、列表消费字段、时间、证据存在语义和每档案图片计数逐项一致；夹具调用生产 DB 读口及图片计数查询，包含权限和序列化。运行 `A4N_TEST_DATABASE_URL=postgres://... cargo test --locked --release -p a4n-db --test object_profile_list_projection -- --ignored --nocapture --test-threads=1`；夹具位于 `crates/platform/a4n-db/tests/object_profile_list_projection.rs`。本轮未测 RSS，不声称内存降幅。

### PERF-063：Agent thread 列表在数据库端过滤和限量

- 调用链与频率：Agent Monitor 首次打开及每 **15 秒**调用一次 `list_agent_threads`，请求当前用户的未归档 thread，最多 **200** 条；同一页面的选中 thread 状态另由事件流更新。基准为当前用户 **10,000** 条历史 thread（其中 **9,800** 条 archived），每条含约 **2 KiB** context summary，实际请求返回 200 条。
- 根因与改动：旧读路径按 `owner_user_id` 和顺序从数据库取回该用户全部 thread，再在应用层排除 archived 项并截断 200 条；返回的 200 项不变，但 9,800 条历史行及其 context summary 被无谓加载和映射。现在把 `archived_at IS NULL`、可选 capability 与 limit 下推到同一数据库查询；owner 范围、`updated_at DESC, id DESC` 排序、应用层 DTO/校验和请求合同保持不变。该 DB 函数唯一生产调用处已同步传入筛选条件。
- 量化结果：隔离 PostgreSQL 16、release（opt-level 3，LTO 关闭）、2 次预热、15 次交错样本；旧/新均走完整 Application 读口并序列化相同响应。P50/P95 **66.578/74.168 → 4.320/6.050 ms**，下降 **93.5%/91.8%**；返回 **200** 条，响应均为 **548,002 B**。物化到应用的 DB 行数 **10,000 → 200**。本轮未测 RSS，不声称进程内存降幅。
- 语义验证与复现：生产 fixture 逐对象及响应字节一致；非忽略测试覆盖 capability 前后空白、archived 开关、过滤后的排序和 limit。日志 `data/performance-audit-20261010/agent-thread-list-projection/command.log`。在隔离 PostgreSQL 16 上设置 `A4N_TEST_DATABASE_URL`，持有 `/home/fufu/Code/A4N/work/.agent-build.lock`，使用 `CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target`、`CARGO_INCREMENTAL=0`、`CARGO_PROFILE_RELEASE_LTO=false` 和 `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`；先执行 `cargo test --locked --offline --release -p a4n-agent-application --lib benchmark_agent_thread_list_pushes_filter_and_limit_into_database --no-run`，再用 `timeout 60s` 执行同一命令并追加 `-- --ignored --nocapture --test-threads=1`。受影响 `a4n-db`/`a4n-agent-application` all-targets check、两项 release 定向测试、`cargo fmt --all -- --check` 和 `git diff --check` 通过；临时 PostgreSQL 与共享 agent target 已清理。

### PERF-064：Manga 页面提示词列表窄读阶段产物

- 调用链与频率：MangaTab 在流水线运行/可取消时每 **2.5 秒**刷新章节状态和页面提示词。基准为 **100** 个页面提示词、**702** 个阶段产物（每页 7 个页级产物，约 **8 KiB** JSON），包含当前 PageSlots、一个 orphaned 提示词和 10 个 stale PageContent 产物。
- 根因与改动：Worker 列表只消费产物的阶段、页码、stale 标记，以及当前非 stale PageSlots payload；旧路径仍从数据库读取并反序列化全部阶段的大 payload。新增列表专用窄投影，数据库只返回上述元数据，并仅为当前 PageSlots 选择 payload；完整产物详情读口不变。提示词字段、排序、stale/orphaned 计算和 PageSlots 错误仍按原规则返回。
- 量化结果：隔离 PostgreSQL 16、release（opt-level 3，LTO 关闭）、2 次预热、15 次交错样本。计时覆盖 Worker 请求校验、项目/Viewer 权限查询、提示词与产物读取、摘要映射和 JSON payload 序列化；未计 HTTP body 解析和外层 Worker envelope。完整操作 P50/P95 **12.980/18.542 → 5.199/7.676 ms**，下降 **59.95%/58.60%**；应用读取的 artifact JSON payload **5,775,481 → 25,178 B**，响应 body **256,692 B** 且旧/新字节完全相同。本轮未测 RSS，不声称峰值内存降幅。
- 语义验证与复现：100 页的 stale/orphaned 列表结果及序列化字节逐轮一致；畸形 PageSlots 在旧/新路径返回相同错误文本。基准夹具 `crates/manga/a4n-manga-record-worker/src/prompts/performance.rs`，日志 `data/performance-audit-20261010/manga-page-prompt-artifact-projection/command.log`。执行 `cargo test --release --locked --offline -p a4n-manga-record-worker --lib performance_manga_prompt_list_artifact_projection -- --ignored --nocapture --test-threads=1`。受影响四个 crates all-targets check、Core 2/2、Worker 2/2、格式检查通过。`a4n-db --lib manga` 为 20 passed、3 failed：两个清理用例的 `data/...png` 夹具路径被项目路径校验拒绝，一个提示词同步顺序断言失败；这些失败位于本次未修改的删除/同步代码路径，未计为通过。

### PERF-065：Agent 消息事件只更新变化的行

- 调用链与频率：Agent Monitor 订阅 message appended/delta/completed/interrupted 事件；每个 `message_delta` 都用新消息对象替换目标 ID，并生成新 `messages` 数组，历史中其他消息对象保持原引用。列表最多显示 **1000** 条消息，流式期间每个事件都会使 `AgentMessageList` 重新分组并协调所有行。
- 根因与改动：PERF-046 只跳过与消息无关的父状态更新；消息数组变化仍调用每条 `AgentMessageRow`。行改为 `React.memo`，比较消息对象、locale、streamed 标记、紧凑模式和配对工具结果对象；行只在其中一项输入变化时重渲染，列表顺序与行数保持原样。
- 量化结果：真实 Chromium **153.0.8010.52**，生产 Vite 夹具；1000 条交替用户/助手历史，分 9 个样本，每样本向尾部助手消息应用 **80 次**新对象内容更新，更新批次之后重置到相同初始输入。三轮交错前后 A/B 的 P50：旧版 **149.2/149.2/150.0 ms**，候选 **18.5/19.4/18.8 ms**；轮次中位数 **149.2 → 18.8 ms（下降 87.4%）**。三轮 1000 个 message row、最终 HTML SHA-256 `9b86859086ff3fe964b663ae17f28aa79353c9035a6979dbbbc291b09de29ea7` 完全一致，消息内容更新和工具结果配对更新断言通过。
- 验证与证据：`npm run test:run -- src/components/agent/AgentMessageRow.test.tsx` **12/12**，`npm run build` 通过（Node 24.21.0；项目要求 Node 22）。基准夹具 `frontend/e2e/performance/agent-messages-fixture.tsx`；对照运行 `A4N_PERF_CASE=agent-messages A4N_PERF_BASELINE_REF=HEAD node e2e/performance/check.mjs` 和同一命令移除 `A4N_PERF_BASELINE_REF`。六份最终报告位于 `data/performance-audit-20261010/agent-message-stream-rerender-final-{before,after}/` 与对应 `*-before-02/after-02`、`*-before-03/after-03/`。早期非确定预热报告在 `agent-message-stream-rerender{,-after}/`，不纳入数字；首个 Chrome 启动失败未保留原报告，摘要补入 `agent-message-stream-rerender/launch-failure.log`，之后短临时目录的 3 轮运行通过。首次临时 DOM 单测因 Vitest Node 环境没有 `document` 未计入验证；工具结果更新断言由真实 Chromium 夹具覆盖。

### PERF-070：健康 Agent 事件流停止重复快照轮询

- 调用链与频率：Agent Monitor 和 Domain Agent Console 初次读取 thread 后订阅 Agent SSE（桌面端为同一游标的事件轮询）；旧实现无论事件流是否健康，都每 **7 秒**读取一次完整 `get_agent_thread` 快照。快照上限为 **1000** 条消息，并包含 runs、executions、approvals、memories 和最近 events。
- 根因与改动：SSE 收到数据块（包括 5 秒 keep-alive 注释）或桌面事件轮询成功后，记录连接活动；流最近 **10 秒**有活动时跳过完整快照。SSE 断开、桌面事件轮询失败或流静默超过 10 秒时恢复 7 秒快照兜底。事件序号、缺口、生命周期事件触发的即时重载、手动操作后的重载和断线重连均保留。SSE API 增加可选活动回调，既有调用方合同不变。
- 量化结果：生产轮询调度函数的假时钟回放，28 秒窗口每 7 秒一次。健康连接旧/新完整快照请求 **4 → 0（100%）**；断开状态仍 **4 → 4**；连接静默 14 秒后下一轮恢复 **1** 次。该基准量化的是重复数据库读取和传输次数，不宣称单次 snapshot 响应时延或 RSS 改善。
- 验证与边界：`agentSnapshotPolling` **3/3**、SSE keep-alive 活动、Agent API/SSE、事件 reducer 和 Domain Agent 相关测试合计 **40/40**；`npm run build`、`git diff --check` 通过（Node 24.21.0；项目要求 Node 22）。超过 10 秒没有任何 SSE 数据块/成功桌面事件读取时恢复快照兜底；事件流本身仍由现有游标重放和生命周期快照校正保证语义。

### PERF-071：Manga 活动流水线只重渲染变化章节

- 调用链与频率：`MangaTab` 在选中章节的 pipeline 为 running/cancelable 时，每 **2.5 秒**读取该章状态并更新 `statuses`；一个项目可显示 **1000** 个章节。原 `MangaChapterList` 每次都在父组件中为全部章节计算状态标签、图标、进度条和按钮 JSX，即使本次只有选中章节的状态变化。
- 根因与改动：把单章行提取为 `React.memo` 组件，逐行接收 chapter、status、stage 和 selected 值；其余章节对象及状态引用不变时跳过行计算。用 ref 代理 `onSelect` 保持行回调稳定，同时始终转发到最近的选择处理函数；章节切换、状态变化和 locale context 变化仍会更新对应行。
- 量化结果：生产组件的 Chromium **153.0.8010.52** A/B，模拟 **1000** 章、预热 **2** 次后每轮 **15** 次单章状态更新，共 **3** 轮。`flushSync` 渲染提交 P50 轮次中位数 **3.8 → 0.6 ms**（下降 **84.2%**），P95 **4.5 → 1.1 ms**（下降 **75.6%**）。两版最终列表 DOM SHA-256 均为 `8480f690387f18a9108ee9820b64831516171b2444ba52eed7b59f65b7fe7ed2`；第 500 章更新到第 51 次状态、点击第 1000 章的行为均通过。
- 验证与边界：夹具 `frontend/e2e/performance/manga-chapter-list-fixture.tsx`；基线源为 `6f9a2d07a`，从 `frontend` 目录持有 `/home/fufu/Code/A4N/work/.agent-build.lock` 运行 `env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy NO_PROXY=localhost,127.0.0.1 A4N_PERF_CASE=manga-chapter-list A4N_PERF_BASELINE_REF=6f9a2d07a A4N_PERF_ARTIFACTS=../data/performance-audit-20261013/manga-chapter-list-before-checked A4N_PERF_PORT=49436 node e2e/performance/check.mjs`，再将 artifacts 目录改为 `../data/performance-audit-20261013/manga-chapter-list-after-checked`、端口改为 `49446` 并移除 `A4N_PERF_BASELINE_REF` 运行候选。真实组件生产构建和列表交互检查通过，最终报告在 `data/performance-audit-20261013/manga-chapter-list-{before-checked,after-checked}/report.json`。耗时是列表状态更新的同步 React/DOM commit，不含 API、浏览器绘制或完整页面交互；未声称网络时延或内存改善。首轮浏览器启动受本机代理变量影响失败，清除 HTTP(S)/ALL proxy 后基线与候选均通过。

### PERF-072：Page Session 过期清理按会话检查并共享周期扫描

- 调用链与频率：Host 进程为每个浏览器 tab 保存一个内存 `PageSession`；前端 heartbeat 约每 **12 秒**一次，每条 SSE 连接原先每 **5 秒**调用一次过期清理。heartbeat、context、manifest、授权、绑定、operation、导航和 control 等 session-ID 请求都先遍历 registry 整张 `HashMap`，之后再处理目标 session。
- 根因与改动：**1000** 个活跃 session 下，1000 次心跳重复扫描约 **1,000,000** 条 registry 记录。现在 session-ID 路径只按 ID 检查并过期目标记录；全表扫描共用 registry clone 间的 5 秒间隔锁，SSE timer 和到期请求最多触发一次周期扫描。过期目标仍即时进入 Offline、取消活动 operation 并发布原来的 offline event；周期 sweep 保留没有请求的 session 清理。`list` 与 `attach_by_code` 保留原来的即时全表扫描。
- 量化结果：`a4n-control` release 生产 `PageSessionRegistry::heartbeat`，每样本在 **1000** 个不同 session 上调用 **1000** 次；每批开始时有一次到期全表 sweep，2 次预热、15 个样本。单批 P50/P95 **100.581/102.547 → 1.087/1.131 ms**，下降 **98.9%/98.9%**。计时包含 registry 更新和 event broadcast，不含 HTTP 身份认证、JSON 编解码和网络；未测 RSS。
- 验证与边界：lease 即时失效、活动 operation 取消及 offline 事件断言通过。`cargo test --locked --offline --release -p a4n-control --lib -- --test-threads=1` **43 passed, 0 failed, 1 ignored**；`cargo fmt --all -- --check`、`git diff --check` 通过。为使既有测试匹配原本实现，修正 `page_navigation_rejects_dirty_editor_context_without_changing_route` 的期望 revision：从 context revision 2 导航后应为 3；导航业务逻辑未改。`list` 与 `attach_by_code` 仍保留各自原有全表 sweep，性能数字仅代表 heartbeat/目标 session 路径。
- 证据：忽略型 release benchmark 位于 `crates/host/a4n-control/src/routes/page_sessions/mod.rs`。在持有 `/home/fufu/Code/A4N/work/.agent-build.lock` 时执行 `CARGO_TARGET_DIR=/home/fufu/Code/A4N/work/.build/agent-target CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 timeout 60s cargo test --locked --offline --release -p a4n-control --lib benchmark_page_session_heartbeat_expiry_scan -- --ignored --nocapture --test-threads=1`；结果分别保存在 `data/performance-audit-20261013/page-session-expiry/baseline.log` 和 `candidate.log`。

### PERF-066：Canvas 下游运行结果失效邻接索引

- 调用链与频率：Video Studio 连线变化、节点配置/引用修改等用户操作会调用 `invalidateCanvasRuntimeResults` 清除源节点及所有可达节点的旧运行结果；节点/边批量删除还会按目标恢复装配剪辑顺序。旧 BFS 每访问一个节点都扫描全部边，`renumberAssembleClipEdges` 又为每个目标过滤全部边，规模大时两者分别是 O(V×E) 和 O(T×E)。
- 根因与改动：下游失效改为一次建立 `source → target[]` 邻接表，BFS 只访问相邻目标；原 visited Set 继续处理环和多路径，空根集合直接返回原数组。装配重排改为一次扫描收集目标装配边，再逐 target 排序并建立新的顺序索引；没有匹配 clip 的 target 不再重复扫描全部边。缺少或无效 `order_index` 仍显性报错。
- 量化结果：真实 Chromium **153.0.8010.52**，生产函数夹具，2 次预热/15 样本、3 轮 A/B。含环、跳边且单根可达所有节点的 1000 节点/2000 边图，下游失效旧版 P50 **7.7/7.5/7.4 ms**，候选 **0.8/0.6/0.7 ms**；轮次中位数 **7.5→0.7 ms（下降 90.7%）**。512 节点失效旧/新轮次中位数 **2.0→0.3 ms（85%）**。批量装配重排使用 **1000** 个节点、**2497** 条边、**500** 个目标需把 `[1,3]` 补为 `[1,2]`；旧版 P50 **10.2/9.4/9.3 ms**，候选 **0.2/0.2/0.2 ms**，轮次中位数 **9.4→0.2 ms（下降 97.9%）**。两种函数输出的 SHA-256 逐项一致，失效节点数与修复目标数均吻合。32 节点结果在约 0.1 ms 计时粒度内，不宣称小图收益。
- 验证与证据：图映射新增环路+共享后代和空根集合测试；装配边测试覆盖跨 target 顺序、缺口修复和错误 `order_index`。Video Studio 两测试文件 **11/11**，`npm run build`、`node --check frontend/e2e/performance/check.mjs`、`git diff --check` 通过（Node 24.21.0；项目要求 Node 22）。夹具 `frontend/e2e/performance/canvas-invalidation-fixture.tsx`；执行 `A4N_PERF_CASE=canvas-invalidation A4N_PERF_BASELINE_REF=HEAD node e2e/performance/check.mjs` 和去掉 `A4N_PERF_BASELINE_REF` 的候选命令。报告位于 `data/performance-audit-20261010/canvas-invalidation-combined-{before,after}-{01,02,03}/`。数字是生产图投影函数 CPU 时间，不代表完整 ReactFlow UI 端到端时延。

### PERF-067：Canvas workflow 计时不重算活动图范围

- 调用链与频率：Video Studio 有 live workflow 时每 **1 秒**更新时间；`workflowNowMs` 只用于每个节点的 elapsed seconds，但旧依赖链据此每秒重建完整 workflow node states、遍历整张图重算 active scope，并重新映射全部边。图含 **1000** 节点、**2000** 边、**1000** 个 checkpoint 时，这些纯派生操作每秒重复一次。
- 根因与改动：新增仅编码节点 ID 和 run status 的 `canvasWorkflowNodeRunStatusSignature`；elapsed seconds 变化不会改签名，因此已有 `useMemo` 复用 active scope 和 `flowEdges` 投影。节点状态转换仍会改变签名并完整重算，边/节点数组变化仍走原路径；每秒的节点 elapsed 投影继续更新。
- 量化结果：Chromium **153.0.8010.52**，实际调用生产 workflow state、active scope、节点类和边类投影；模拟 **30** 次 1 秒更新，包含 1000 checkpoint 状态映射和全部 flow node 投影。3 轮旧/候选 P50 **41.3/39.0/40.0 → 26.3/25.4/25.7 ms**，轮次中位数 **40.0→25.7 ms（下降 35.8%）**；每轮最终节点/边投影 SHA-256 相同。新旧数字是 30 个 timer tick 的投影 CPU 累计，未测实际 ReactFlow reconciliation/完整用户感知端到端延迟。
- 验证与证据：`canvasWorkflowState.test.ts` **4/4**，包括 elapsed 秒变化时签名稳定、状态变更时签名变化；`npm run build` 通过（Node 24.21.0；项目要求 Node 22）。夹具 `frontend/e2e/performance/canvas-workflow-timer-fixture.tsx`，运行 `A4N_PERF_CASE=canvas-workflow-timer node e2e/performance/check.mjs`；三份报告位于 `data/performance-audit-20261010/canvas-workflow-timer-{01,02,03}/`。

### PERF-068：大纲进度投影按实际记录处理章节范围

- 调用链与频率：`OutlineAdvancedPanel` 在启用自动刷新时每 **3 秒**调用 `getGenerationProgress`；Control 经过 Runtime 调用 Outline Worker，Worker 先检查 Viewer 权限，再串行读取分部范围和章节大纲的窄列并生成四个计数。数据库查询只返回 `id/start/end` 与 `id/chapter/status`。
- 根因与改动：旧投影为检测分部重叠，把每个 `start_chapter..=end_chapter` 中的章节号全部写入 `HashSet`，再逐号查询章节状态；一条百万跨度的分部即使没有章节大纲，也会扩展一百万个整数。现在使用按区间组织的 `BTreeMap` 校验重叠，完成统计将 SQL 已按章节号排序的完成章节与已排序分部区间线性合并；不再按跨度扩展范围。章节状态、重复章节、无效范围和重叠仍返回冲突，摘要合同和计数不变。
- 量化结果：Rust release 生产投影函数，**3** 次交错复跑，每次 **2** 次预热、**15** 个样本。典型夹具为 **1000** 章、**20** 部、**800** 章完成；三轮 P50/P95 中位数 **56.044/57.194 → 2.039/2.155 µs**（下降 **96.36%/96.23%**）。稀疏边界夹具为 **1** 个 `1..=1,000,000` 分部、**0** 条章节大纲；P50/P95 中位数 **30.518/30.764 ms → 0.175/0.273 µs**（下降超过 **99.999%**）。两种夹具旧/新 `OutlineGenerationProgressSummary` 完全相同。
- 验证与边界：`a4n-outline-worker --lib` 正确性测试 **3/3**、忽略型 A/B benchmark **3/3**；覆盖大跨度无大纲、未排序分部、重复章节和重叠分部。`cargo fmt --all -- --check` 与 `git diff --check` 通过。额外的 `--all-targets --no-run` 未完成：共享 agent target 在 `a4n-db` 编译期间消失；随后在锁内重新编译并通过 `--lib`。当前环境未设置 `A4N_TEST_DATABASE_URL`，没有测完整 HTTP/数据库往返或 RSS；上述数字只代表每次轮询中的 Worker 投影 CPU，不能写作整条 API 时延。
- 证据：基准在 `crates/planning/a4n-outline-worker/src/projection.rs` 的忽略型 release 测试中；执行 `cargo test --locked --offline --release -p a4n-outline-worker --lib benchmark_generation_progress_projection -- --ignored --nocapture --test-threads=1`。本次复跑摘要见 `data/performance-audit-20261010/outline-progress-projection/benchmark.log`。

### PERF-069：未归纳章节跳过无用来源修订哈希

- 调用链与频率：Fact Feedback 面板在工作流运行时每 **1.5 秒**读取一次 `get_project_fact_feedback`；摘要调用 `current_feedback_sources` 读取项目当前章节和已应用的章节反馈记录。旧实现对每章先哈希正文、序列化并哈希分析 JSON，再检查是否存在匹配的已应用记录；未归纳章节的修订永远不会返回。
- 根因与改动：用已应用记录数量作为安全下界：只有记录行少于当前章节数时，才能确定至少存在未覆盖章节；此时从原有 `(chapter_id, after_json)` 集合建章节 ID 集，缺少记录的章节在 `source_revision` 前跳过。完整覆盖和历史记录较多的情况仍走原 BTreeSet 精确比较；匹配到当前修订后，world_facts 仍按原路径解析并验证。没有改变 DTO、权限、错误或轮询频率。
- 量化结果：隔离 PostgreSQL 16、release opt-level 3（LTO 关闭）、1000 章、每章 **9 KiB** 正文和 **8 KiB** 分析数据，2 次预热/15 次交错样本，计时 `current_feedback_sources` 读取与来源构造。0 条已应用来源：P50/P95 **42.941/55.226 → 19.717/30.651 ms**（54.1%/44.5%）；100 条来源：**37.927/49.480 → 22.453/24.831 ms**（40.8%/49.8%）；1000 条完整来源：P50 **50.595 → 50.254 ms**（0.7%，视为持平），P95 **60.578 → 75.881 ms**（增加 25.3%，不宣称完整覆盖有收益）。旧/新完整 `FeedbackSource` 对象逐字段相同。
- 验证与边界：忽略型 benchmark 三种覆盖率 **1/1** 通过；现有 `body_fact_feedback::partial_feedback_is_versioned_source_fenced_and_reversible` **1/1** 先前通过。当前候选在 release、opt-level 3、LTO 关闭下重新编译并运行；`cargo fmt --all -- --check` 和 `git diff --check` 通过。一次全新 release 测试 harness 编译超过总命令 60 秒限制，未进入测试体且不计为结果；解除编译超时后构建完成，实际 benchmark 仍限时 60 秒并通过。指标只代表 `current_feedback_sources` DB/CPU 投影，不代表完整 API 往返或 RSS；反馈覆盖分布的线上均值未知。
- 证据：`data/performance-audit-20261010/fact-feedback-source-prefilter-rerun-20261013/command.log`、`fact-feedback-source-prefilter-candidate/command.log` 和 `fact-feedback-source-prefilter-regression/command.log`。忽略型基准源码在 `crates/platform/a4n-db/src/fact_feedback/tests.rs`。

### 拒绝候选：Fact Feedback 摘要合并重复章节读取

- 根因候选：Fact Feedback 摘要并行读取 `current_feedback_sources` 和 `chapters_for_project`；第二个读口只使用章节 ID、章节号和可用正文标记，但旧查询会再次取回分析 JSON 等完整章节字段。候选让来源读取同时产出可用章节索引，避免第二次章节扫描。
- 量化结果：隔离 PostgreSQL 16、release opt-level 3（LTO 关闭）、1000 章、每章 **9 KiB** 正文和 **8 KiB** 分析，2 次预热/15 次交错；计时上述章节读取分支，不代表完整 API。0 条已应用来源时 P50/P95 **29.995/38.089 → 18.589/21.498 ms**（下降 **38.0%/43.6%**）；100 条时 **34.936/41.552 → 30.713/39.158 ms**（下降 **12.1%/5.8%**）；1000 条时 **58.211/69.953 → 46.679/49.411 ms**（下降 **19.8%/29.4%**）。三种覆盖下来源记录和可用章节索引逐项一致。
- 决定：只有 0 条来源的分布超过 30% 时延门槛；100 条和完整覆盖未达标，且没有完整 `fact_feedback_summary_for_user` 端到端基准证明 API 达标。当前不保留候选代码，以免把仅在空覆盖下的收益外推到正在反馈流程通常遇到的覆盖分布。基准日志 `data/performance-audit-20261010/fact-feedback-summary-chapter-read-candidate-20261013/command.log`；原摘要路径和业务代码保持原实现。

### 拒绝候选：静态语言字典按 locale 拆分

- 调用链与规模：`main.tsx → I18nProvider → translate.ts` 在初始入口静态加载中文和英文两份字典，源码合计 **728,535 B**。当前 production 入口 `.js.gz` 为 **328,672 B**。
- 上界测量：用只加载中文字典的临时 production build 估算移除另一 locale 的最大收益（该构建把 `en-US` 暂时映射到中文，仅供测量，不保留）。入口由 **1,065,828 → 732,800 B raw**、**328,672 → 234,160 B gzip（28.8%）**。Chromium 153 新建 5 个会话并在首次登录页交互后强制 GC，heap 中位数 **3,108,764 → 2,676,796 B（13.9%）**。实验语言内容不匹配会让候选页面更短，因此内存降幅只作上界参考。
- 决定：上界 bundle 降幅仍低于 30%，heap 降幅低于 20%；真正的按 locale 动态加载还必须在显示本地化 UI 前取回所选字典，并增加一次关键请求，没有证据证明完整页面响应能达到 30% 门槛。未保留产品改动。构建环境 Node **24.21.0**（项目要求 22）；本项仅做构建体积和 heap 对照，没有声称页面响应时延改善。

### 拒绝候选：图片图库元数据索引

- 链路与规模：Novel Detail 的 Images 页进入项目图库时读取全部图片摘要；候选项目为 **1000** 张图片、每条约 **8 KiB** `metadata_json`。页面仍需要提示词、路径、收藏、页码和源内容哈希，详情操作需要完整元数据。
- 候选：数据库列表仅读取卡片字段，并通过 JSON 标量提取 `page_number` 与 `source.content_sha256`；完整元数据改为按图片 ID 详情读取。旧完整读取与候选索引逐项核对卡片字段、分组页码和 stale 源哈希。
- 量化结果：隔离 PostgreSQL 16、release、15 次交错样本、2 次预热。完整/索引响应 **8,584,691 → 348,691 B**；P50/P95 **11.410/18.936 → 8.601/11.447 ms**，下降 **24.6%/39.5%**。P50 未达到 30% 门槛，且没有独立进程 RSS 下降 20% 证据，候选撤回。
- 证据与决定：`crates/platform/a4n-db/tests/generated_image_list_projection.rs` 和 `data/performance-audit-20261010/generated-image-list-projection/command.log` 保留基准。当前 `list_images_by_project` 合同与 Images 页保持原样；后续只有在真实图库规模或写入侧拆分元数据列有新证据时再评估。

### 拒绝候选：Canvas 文本素材窄索引

- 链路与规模：`ProjectTextMaterialsPanel` 初载 **256 份**文本素材，每份正文约 **4 KiB**；旧列表响应包含全部正文，而页面只展示目录并读取当前选中版本。候选增加不含正文的索引命令，切换素材仍通过版本接口读取正文，并验证第二份素材内容和标题。
- 量化结果：真实 Chromium 153 受控三轮中位数首屏 **128.2 → 110.2 ms（14.0%）**；GC 后 JS heap **38.30 → 35.16 MB（8.2%）**；请求 JSON **3,206,959 → 65,583 B（98.0%）**。首屏时间和内存均未达到门槛，候选撤回。
- 语义验证：旧/新均保留 256 项、首项和第二项正文各 4,096 字符、第二项标题一致；仅保留夹具和报告，不改 API 合同、worker、数据库或面板。
- 证据：`data/performance-audit-20261010/text-materials/{before-4,before-5,before-6,after-1,after-2,after-3,before-7,before-8,before-9,after-5,after-6,after-7}/report.json`；该受控数据说明载荷浪费，但不足以证明满足本项目保留标准。

### PERF-048：媒体改编阶段标签索引化

- 调用链与规模：Manga/Galgame 页面构造阶段标签时，未选择单章会以所有章节作为 scope；旧实现对每个 scope 过滤全部同类文档，并在缺少当前快照文档时再次扫描文档集合。基准为 **1000 章、1000 份文档、80 次**与标签无关的状态更新。
- 根因与改动：标签只需要每个 scope 的当前快照 draft/adopted 文档和是否存在任意历史文档。一次遍历建立两个 Map/Set，按原 draft 优先、adopted 次之、stale/需更新判断和完成计数输出；未引入缓存或改变文档读取。
- 量化结果：真实 Chromium **153.0.8010.52**、生产构建、三轮 A/B 中位数 **100.3 → 6.2 ms（下降 93.8%）**；1000 章、1000 文档和“已应用”标签均保持一致。
- 语义验证：`useMediaAuthoring.test.ts` 3 项覆盖 draft 优先、旧快照需更新和 Galgame `null` scope；夹具检查章节/文档数量及最终标签。
- 证据与复现：`data/performance-audit-20261010/authoring-label/{before-1,before-2,before-3,after-1,after-2,after-3}/report.json`；从 `frontend` 执行 `env -u http_proxy -u https_proxy -u all_proxy -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u NODE_USE_ENV_PROXY A4N_PERF_CASE=authoring-label A4N_PERF_BASELINE_REF=HEAD A4N_PERF_ARTIFACTS=<独立目录> A4N_PERF_PORT=<空闲端口> node e2e/performance/check.mjs`，新版去掉 `A4N_PERF_BASELINE_REF`。

### 拒绝候选：项目包 workflow checkpoint 批量导入

- 链路与规模：用户导入项目包历史时，`import_project_bundle_history` 逐条校验 checkpoint contract、原始输入指纹、递归 ID 映射和升级回执，再在同一事务中写入 checkpoint；基准为 **128 条** checkpoint、输入 JSON **3,217,197 B**。
- 候选：保留全部逐条转换和校验，仅把已转换的 ActiveModel 按现有 1,000 行批次 `insert_many` 写入。
- 量化结果：两轮同库交错 A/B 的 P50/P95 分别为 **70.703/124.546 → 44.487/66.161 ms（37.1%/46.9%）**、**47.221/58.071 → 35.944/44.202 ms（23.9%/23.9%）**；第二轮未达到 30% 门槛，收益不稳定，候选已撤回。恢复后的正式入口字段、映射后 JSON、指纹和顺序逐项一致；基准输出保留在 `data/performance-audit-20261010/workflow-checkpoint-import-{before,after,after-repeat}/`。
- 决定：不改变 checkpoint 的逐条写入路径。输入 JSON 体积和指纹/升级校验成本需要更多代表性数据后再评估；当前夹具保留用于后续复测，不把首轮尾延迟降幅计入性能收益。

### 拒绝候选：故事事件行 DOM 压缩

- 链路与规模：故事事件页初次渲染及筛选更新，基准为 **1000 章/5000 个事件**，连续执行包含收窄和放宽的 8 次筛选；每轮覆盖 1440/390px、Enter 打开章节和横向溢出检查。
- 候选：将事件 bullet 与元数据分隔点改为 CSS 伪元素，并合并事件行的无意义包裹节点。字段顺序、重要性颜色、章节跳转和打印容器保持不变；分隔点使用 `::before/::after` 后不进入屏幕阅读器文本。
- 量化结果：真实 Chromium 153 三轮交错中位数（1000 章）交互总耗时 **421.1 → 360.5 ms（14.4%）**，GC 后 JS heap **29.01 → 25.46 MB（12.2%）**；DOM 元素 **69,008 → 39,008（43.5%）**。前两项未达到本项目 30% 时延或 20% 内存保留门槛，产品候选撤回。
- 语义验证：三轮旧/新语义文本 SHA-256 分别为 `144f2d96682153aecececec18dcf867402034de132c19ad113a19eb396cb6124`，事件行数、章节末端位置、两种视口截图和键盘路径一致；候选视觉 HTML 哈希不同仅来自装饰节点重排。夹具保留规范化语义哈希，避免后续重复评估时误把 CSS 装饰当业务文本。
- 证据：`data/performance-audit-20261010/story-event-markup-final/{before-01,before-02,before-03,after-compact-01,after-compact-02,after-compact-03}/report.json`；产品代码恢复为基线实现，本条仅记录拒绝结论和夹具校验。
