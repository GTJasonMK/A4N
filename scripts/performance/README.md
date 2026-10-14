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
| [PERF-077](../../crates/host/a4n-orchestrator/src/fact_feedback/mod.rs) | 事实反馈面板仅在工作流运行时每 1.5 秒轮询；1000 章、每章 9 KiB 正文与 8 KiB 分析、每项目 10,161 条变更历史 | 章节修订与可用章节共用一次投影；历史通过部分索引取最近 ID，再读取至多 100 个完整行并保留原 Rust 等值判断 | Orchestrator 摘要 P50 **80.8/86.3/93.5 → 43.9/45.5/53.5 ms（45.7%/47.3%/42.8%）**；三种覆盖率下 DTO 和响应字节相同 |
| [PERF-078](../../crates/platform/a4n-db/src/project/source_snapshot/queries.rs) | 漫画／视频／Galgame 派生工作台初载与刷新；1000 章、每章 9 KiB 正文和 8 KiB 分析，漫画/视频默认选中一章 | 列表改为 id/snapshot_id/源章 ID/章号/标题投影；漫画与视频只按项目及当前源章 ID读取一条完整快照，Galgame仅用索引 | 旧全量列表 **17,913,472 B / P50 41.06 ms / P95 47.39 ms**；索引+当前章 **109,482 B / P50 3.09 ms / P95 6.02 ms**；延迟分别下降 **92.48% / 87.30%**，字节下降 **99.39%** |
| [PERF-079](../../crates/platform/a4n-db/src/collaboration/access/info.rs) | 项目列表／摘要 → 协作摘要；100 个项目、每项目 1000 成员 | 成员数和当前用户角色在 PostgreSQL 分组聚合；不再将所有成员行解码到 Worker | 100,000 行时查询 P50 **29.540 → 19.399 ms（34.33%）**，结果字节 **1,381,300 → 1,492（99.89%）**；这是成员聚合查询，不是完整 API 响应 |
| [PERF-080](../../frontend/src/utils/videoPlayback.ts) | 每次选中／重载视频 → Host 内联 Base64 → `<video>`；单个文件最高 256 MiB | 删除仅用于校验的前端整串 `atob`；Host 已用 `base64::STANDARD` 编码，播放元素是唯一必要解码者 | Chromium 153、64 MiB：helper P50 **124.7 → 0.1 ms**，临时 JS heap **67,108,955 → 4,836 B**；分别下降 **99.92% / 99.99%** |
| [PERF-081](../../crates/media/a4n-media/src/upload.rs) | 视频源上传 → Runtime → 临时文件；64 MiB 基准、接口上限 512 MiB | 复用原有空值/长度/Base64 校验，以 `DecoderReader` 分块写入临时文件，避免完整解码 `Vec<u8>` | 固定 CPU 的 release A/B P50 **100.975 → 98.010 ms（2.94%）**；进程峰值 RSS **151.562 → 87.770 MiB（42.09%）**；字节相同 |
| [PERF-082](../../frontend/src/pages/novel-detail/video/VideoSegmentPlanEditor.tsx) | 长视频分镜编辑；每次文本输入更新整份 segment plan，50／100／200 段 | 路径级不可变更新保留未修改段引用，memoize 未变分段，dirty 检查复用固定基线序列化 | 固定 CPU 18/19、Chromium 153，每 8 次输入总耗时 P50：50 段 **211.5 → 11.6 ms（94.52%）**、100 段 **459.7 → 17.0 ms（96.30%）**、200 段 **976.9 → 24.2 ms（97.52%）**；保存摘要完全相同 |
| [PERF-083](../../frontend/src/pages/novel-detail/shooting-script/StructuredScriptEditor.tsx) | 影片剧本编辑；每次镜头字段输入更新完整剧本，25／50／100 镜头 | 共用 `updateAtPath` 只复制修改路径，稳定更新回调并 memoize 未变化的 ShotEditor | 固定 CPU 18/19、Chromium 153，每 8 次输入总耗时 P50：25 镜头 **183.6 → 34.8 ms（81.05%）**、50 镜头 **385.8 → 60.7 ms（84.27%）**、100 镜头 **775.8 → 103.5 ms（86.66%）**；保存摘要完全相同 |
| [PERF-084](../../frontend/src/pages/derived/MediaPlanEditors.tsx) | Galgame 分支计划编辑；每个输入重算所有章节改编单元/节拍，500／1000 章 | 以章节 ID Map 替代逐章扫描全部文档的嵌套 `find`，保留文档首个匹配顺序 | 固定 CPU 18/19、Chromium 153，每 8 次输入总耗时 P50：500 章 **30.9 → 11.5 ms（62.78%）**、1000 章 **77.2 → 18.1 ms（76.55%）**；计划摘要相同。100 章为 **11.8 → 12.3 ms**，不计为收益 |
| [PERF-085](../../crates/platform/a4n-db/src/media_authoring.rs) | Galgame 分支计划保存／漫画计划校验 → 1000 个章节依赖；每份文档约 8 KiB | 按 project 和依赖 ID 分批读取，最多 64 份完整记录；逐条 canonical hash/revision/status/kind 校验 | 隔离 PostgreSQL 16、CPU 18/19、5 组交错（2 组预热、3 组计量）P50 **237.318 → 79.038 ms（66.7%）**；JSON **8,291,000 B**，校验结果相同 |
| [PERF-086](../../crates/platform/a4n-db/src/media_authoring.rs) | 多章节 Galgame 分支计划保存 → 验证 1000 个来源快照章节；正文共 **8,192,000 B** | 按来源快照章节 ID 每批读取最多 64 行；按输入顺序保留来源身份、正文哈希、重复／缺失错误与独立漫画映射检查 | 隔离 PostgreSQL 16、CPU 18/19、5 组交错（2 组预热、3 组计量）P50 **189.626 → 28.061 ms（85.2%）**；内容校验结果相同 |
| [PERF-087](../../crates/host/a4n-orchestrator/src/galgame/read_models/pipeline.rs) | Galgame 千章 context 初装与分支阶段 → 1000 个已采用计划；每份文档约 8 KiB | draft/adopted 当前文档按章批读；分支阶段按计划 ID 批读；仍逐条解析并验证 draft 优先级、记录身份和 revision | 隔离 PostgreSQL 16、CPU 18/19、5 组交错（2 组预热、3 组计量）P50：context **1,037.152 → 95.208 ms（90.8%）**；分支阶段刷新 **252.101 → 80.963 ms（67.9%）**；摘要顺序与状态相同 |
| [PERF-088](../../frontend/src/pages/novel-detail/visual-plan/useDirectorPlan.ts) | ChapterVisualPlan 每次镜头输入 → dirty 检查、编辑 revision 与本地草稿备份；100／500／1000 段、每份计划 82,583／416,583／834,088 B | 每次更改只序列化一次；dirty、revision、同步备份共用同一表示；效果阶段跳过同一对象的重复 sessionStorage 写入 | Chromium 153、CPU 18/19、8 次输入 P50：100 段 **16.8 → 9.7 ms（42.3%）**、500 段 **114.0 → 56.4 ms（50.5%）**、1000 段 **234.2 → 112.3 ms（52.1%）** |
| [PERF-091](../../crates/host/a4n-orchestrator/src/media_authoring/application.rs) | 手动首次保存 Galgame 分支计划 → 收集 1000 章 adopted 改编依赖；每份文档约 8 KiB | 按 project/snapshot/source chapter 每批 64 条仅读 adopted 计划；调用方继续按章节顺序 parse 并生成 reference | 隔离 PostgreSQL 16、CPU 18/19、5 组交错（2 组预热、3 组计量）P50 **475.357 → 100.909 ms（78.8%）**；依赖 ID、revision、hash、status 顺序相同 |
| [PERF-092](../../crates/host/a4n-orchestrator/src/galgame/webgal_export_application.rs) | WebGAL 导出选择合同 → 1000／5000／10000 场景、每场 2 个选项 | 用现有导出 scene ID Map 校验目标；删除 Map 命中后对详情场景的第二次线性扫描 | Rust release、CPU 18/19，完整合同验证 P50 **2.394→0.288 ms（88.0%）／51.907→1.604 ms（96.9%）／202.676→4.577 ms（97.7%）**；结果与错误语义相同 |
| [PERF-093](../../crates/platform/a4n-db/src/project/dependency_graph/builder/topology.rs) | 资产依赖图请求 → `GraphBuilder::finish`；8 层 DAG、1000／5000／10000 节点，平均 2.625 边/节点 | 节点 ID 映射到整数索引；使用 Vec 邻接、可复用队列和访问代次，保留精确唯一后代数 | Rust release、CPU 18/19，P50 **94.467→5.335 ms（94.35%）／1155.143→37.748 ms（96.73%）／2735.239→84.155 ms（96.92%）**；后代数与图摘要一致 |
| [PERF-094](../../frontend/src/pages/novel-detail/asset-graph/graphCanvas.ts) | 打开／刷新资产依赖图 → `buildLayout`；8 层 DAG、1000／5000／10000 节点、2625／13125／26250 边 | 每个可见节点只计算一次聚类排序键，供拓扑堆和五轮分层排序重复比较使用 | Chromium 153.0.8010.52、CPU 18/19、生产 Vite P50 **53.400→25.700 ms（51.87%）／345.600→172.600 ms（50.06%）／781.700→403.100 ms（48.43%）**；坐标与层级摘要完全相同 |
| [PERF-095](../../frontend/src/pages/novel-detail/MangaTab.tsx) | NovelDetail 根项目漫画页初载 → 章节目录；1000 章 | 目录改用已存在的 `listChapterIndex`；只有当前章节选中时才读完整 `getChapter` | 复用 PERF-017 同一真实 1000 章列表 API 对照：完整章节响应 **11,601,686→742,686 B（减少 93.60%）**；多章节初载不请求正文/分析 |
| [PERF-096](../../frontend/src/pages/novel-detail/MangaTab.tsx) | 根项目 MangaTab 多章空态 → 角色档案、立绘、全项目图片读取；1000 章、5000／10000 图片 | 没有选中章节时不读取详情面板未使用的三组项目资产；首次选择或单章自动选择时仍读取 | Chromium 153、生产 Vite、4× CPU、4 Mbps/50 ms、三轮：JS heap P50 **14.76→9.01 MB（38.96%）／19.03→9.01 MB（52.65%）**；解压 API JSON **5,722,957→969,812 B（83.05%）／10,274,345→969,812 B（90.56%）**；目录就绪仅快 **7.21%／8.01%**，不计为时延收益 |
| [PERF-097](../../frontend/src/pages/novel-detail/manga/ReferenceBindingsPanel.tsx) | 漫画参考图绑定器每次打开／搜索；3 个角色、50 档案、200 立绘、5000／10000 项目图片 | 超过 100 个候选时改为关键词过滤并最多渲染 100 项；候选保留稳定 ID/面板身份，选择后预览缩略图；小列表沿用原下拉 | Chromium 153、生产 Vite、三轮交错：候选初载 P50 **59.6→9.5 ms（84.06%）／98.8→8.3 ms（91.60%）**；宽泛 `page` 搜索并打开 **329.1→99.6 ms（69.74%）／576.0→114.7 ms（80.09%）**；GC 后 JS heap **8614→5005 KiB（41.90%）／13738→6625 KiB（51.78%）**；指针/键盘可添加，1440/1024/390px 无溢出 |
| [PERF-098](../../frontend/src/pages/novel-detail/visual-plan/timeline.ts) | ChapterVisualPlan 拆分镜头弹窗每次重渲染 → 计算段内时间点；1000／5000 点和段 | 将循环内反复查找目标段与起止点收敛为各查找一次，再执行相同的段内过滤 | Chromium 153、生产 Vite、2 轮预热＋7 轮交错：1000 项 **10.5→≤0.1 ms**；5000 项 **78.6→0.1 ms（99.87%）**；输出点数、ID 与顺序逐项一致 |
| [PERF-099](../../frontend/src/pages/novel-detail/visual-plan/timeline.ts) | ChapterVisualPlan 时间点选中／编辑源投影 → 查找所属分镜；1000／5000 点和段 | 一次建立保留首个 ID 的时间点 Map，再按原段顺序扫描并返回首个包含时间的分镜；保留缺失端点错误与总时长边界 | Chromium 153、生产 Vite、2 轮预热＋7 轮交错：靠后段查找 P50 **4.8→≤0.1 ms／32.6→0.2 ms（99.39%）**；选中分镜 ID 完全一致 |
| [PERF-101](../../frontend/src/pages/novel-detail/images/ImageGalleryPanel.tsx) | 项目图片图库初载；1000／2000 张图片 | 首次仅挂载 500 张卡片，底部按钮按批显示其余图片；过滤和范围切换重置批次 | Chromium 153.0.8010.52、生产 Vite、CPU 18/19、3 轮交错：挂载 P50 **192.1→111.0 ms（42.22%）／339.8→113.9 ms（66.48%）**；首屏后 JS heap 降 **27.08%／51.81%**；图片请求与传输减少 **50%／75%**；完整图片列表仍可逐批到达 |
| [PERF-102](../../crates/platform/a4n-migration/src/migrations/generated_image_project_index.rs) | 项目图库 API → `generated_image` 按项目查询；100,000 行、每项目 1,000 张 | 为现有 `WHERE project_id = ?` 增加单列数据库索引；保持原 `created_at DESC` 排序和完整 DTO 投影 | 隔离 PostgreSQL 16、5 轮交错、服务器 `EXPLAIN ANALYZE`：P50 **14.223→4.265 ms（70.01%）**；计划从全表扫描变为项目索引 Bitmap Heap Scan；索引 **745,472 B**；结果 1,000 个 ID 顺序完全一致 |
| [PERF-103](../../crates/platform/a4n-migration/src/migrations/generated_image_project_chapter_index.rs) | 章节图库 API → `generated_image` 按项目和章节查询；100,000 行、目标项目/章节 100 张 | 为现有 `WHERE project_id = ? AND chapter_id = ?` 增加复合数据库索引；保持原 `created_at DESC` 排序和完整 DTO 投影 | 隔离 PostgreSQL 16、5 轮交错、服务器 `EXPLAIN ANALYZE`：P50 **17.994→0.812 ms（95.49%）**；计划从全表扫描变为复合索引 Bitmap Heap Scan；索引 **761,856 B**；结果 100 个 ID 顺序完全一致 |
| [PERF-105](../../crates/platform/a4n-migration/src/migrations/workflow_run_project_updated_index.rs) | 工作流列表按项目读取最新运行；100,000 行、目标项目 20,000 行、默认返回 100 条 | 为现有 `WHERE project_id = ? ORDER BY updated_at DESC, id DESC LIMIT 100` 增加排序复合索引；保持完整运行 DTO 和排序语义 | 隔离 PostgreSQL 16、已有项目/状态索引、5 轮交错：P50 **7.702→0.292 ms（96.21%）**；计划从 Bitmap Heap + Sort 变为 Limit + Index Scan；索引 **4,079,616 B**；100 个 ID 顺序完全一致 |
| [PERF-106](../../crates/platform/a4n-migration/src/migrations/task_queue_project_created_index.rs) | 任务历史分页按项目读取；100,000 行、目标项目 20,000 行、默认返回 100 条 | 为现有 `WHERE project_id = ? ORDER BY created_at DESC LIMIT 100` 增加项目/时间排序索引；保持完整任务 DTO 和分页顺序 | 隔离 PostgreSQL 16、5 轮交错：P50 **15.588→0.412 ms（97.36%）**；计划从 Limit + Seq Scan + Sort 变为 Limit + Index Scan；索引 **3,178,496 B**；100 个 ID 顺序完全一致 |

较小输入同样保留记录：OpenAI 1/32/256 事件分别下降 6.66%/6.74%/26.08%，Anthropic 分别下降 7.45%/10.30%/35.55%。1000 字符、995 次显示更新为 27.4 → 20.1 ms（26.64%）。这些没有达到 30% 的小输入结果不作为主要验收收益，不把压力批次的提升外推到单事件请求。

Intl 的四组对照（每组 2000 次）：中文数字 75.7 → 2.3 ms、中文相对时间 69.3 → 2.9 ms、英文数字 74.8 → 2.3 ms、英文相对时间 78.3 → 2.7 ms。自定义格式选项和日期时区路径没有改变。

## 审查覆盖与未修改项

| 范围 | 已核对事实与决定 |
| --- | --- |
| 首屏和路由 | `App.tsx` 已按路由和依赖图弹窗懒加载。PERF-009 已通过真实 Edge 和生产登录页验证压缩交付；同一份入口文件从 1,010,285 原始字节变为 310,789 传输字节。两种语言字典仍静态加载，未宣称这一部分代码拆分已经完成。 |
| 项目列表 | `ProjectReadWorker::summary_for_projects` 已批量查询项目族、权限、进度和反馈间隔；不重新引入逐项目请求或重复批处理层。 |
| 章节列表 | PERF-011 消除未使用历史读取及检查点深复制，PERF-017 将写作桌/只读目录/导入预览改为按需读取；PERF-095 将根项目 MangaTab 也改为索引加按需详情。原 `ChapterSummary` 完整合同保留；故事事件 PERF-019 和派生来源快照 PERF-078 已分别优化。 |
| 数据库权限和协作 | 读取使用连接池、绑定参数和批量根项目查询。PERF-079 将高基数成员统计收敛为 DB 聚合；未过期邀请统计单独 A/B 仅改善 8.23%，低于门槛，未改写实现或增添索引。 |
| 工作流轮询和恢复 | 旧任务进度轮询间隔 500 ms，已有单次进行中保护和结束清理；新生成任务走事件订阅。未通过减少进度频率来换取表面收益。恢复快照的读取需求与公开进度投影分开评估。 |
| HTTP 与媒体 | Worker／service client 已持有并复用 HTTP client；Web 媒体使用 `ServeFile`，Edge 使用流式 body。桌面自定义协议要求字节响应，不能直接套用 Web 的传输方案。PERF-080 去掉播放器 data URL 前多余的完整 Base64 解码；PERF-081 限制上传解码的峰值内存，不改 API 载荷。 |
| LLM 上游连接 | PERF-012 保留每次构造 provider 的配置与凭证隔离，仅延长 HTTP Client 的生命周期。原连接／逐次读取超时、错误、取消与背压检查通过；不缓存模型响应。 |
| 嵌入模型生命周期 | PERF-014／015 已处理本地模型和 API 连接的重复初始化。模型文件删除／下载会使额外持有的资源失效；仍存活的 provider 保留原有拥有权，避免破坏已在使用的模型。没有缓存业务向量结果。 |
| 语义分块 | 已复用向量范数并利用相似矩阵对称性；本次只消除额外矩阵。将全矩阵换成局部带状算法可能改变浮点抵消和相同分数时的切分，未作为无风险替换。 |
| 故事事件 | PERF-018 减少筛选时未变化卡片的重复渲染；PERF-019 从数据库到前端仅读取所需的五个分析字段。千章/4500事件仍需渲染全部可变高卡片；原生 `content-visibility:auto` 已跳过屏外布局，但页面整体只改善约12.5%。进一步虚拟化会改变屏外 DOM/打印行为，当前没有可复用实现和整页 A/B 证据证明达到30%门槛，因此本轮不改变列表行为。 |
| 漫画与派生媒体来源读取 | 漫画、视频、Galgame 工作台初载/刷新都使用源快照列表；列表只显示章号/标题，漫画与视频仅当前选中章需要正文。PERF-078 将产品列表改为窄索引，保留项目/快照归属的单章读口；Galgame 全程只取索引，旧完整列表 API 保留兼容。 |
| 视频分镜编辑 | PERF-082 在每次文本输入时按编辑路径复制并保留未变化段引用，避免全量深拷贝和全段重渲染；完整 dirty/revert 语义与保存内容保持一致。 |
| 影片剧本编辑 | PERF-083 在每次输入时复制整份场景/镜头数据，改为路径级复制；未变化的镜头子树复用，源剧本和保存数据保持一致。 |
| Galgame 分支计划编辑 | PERF-084 在每次输入时按章逐个扫描全部创作文档，改为线性构建章节计划索引并映射可选单元/节拍，保留首个有效采用版本的顺序。 |
| 媒体创作依赖校验 | PERF-085 复用项目/依赖 ID 批量读口，单批最多 64 条完整记录；返回后仍逐项核对引用顺序、哈希、revision、状态、类型和缺失项。 |
| 媒体创作来源校验 | PERF-086 将多章来源快照主键串行读取改为每批 64 行；保留逐来源完整身份与正文哈希校验，单章漫画校验路径不变。 |
| Galgame 流程文档读取 | PERF-087 将千章 context 的 draft/adopted 两阶段逐章查找和分支阶段按 ID 重读，改为有界批次读取；调用方仍按原顺序执行文档解析和版本校验。 |
| ChapterVisualPlan 编辑 | PERF-088 复用当前 plan 的精确 JSON 表示，避免每次编辑分别做 dirty 比较、revision 比较和两次相同本地备份序列化/写入；同步备份仍保留，其他替换仍由 effect 备份。 |
| ChapterVisualPlan diff 候选 | PERF-089（PERF-088 基线）对共享对象引用相同的点/镜头跳过 JSON 深比较；100／500／1000 段 P50 **9.1→7.1 ms（22.0%）／56.1→44.3 ms（21.0%）／109.3→88.9 ms（18.7%）**，均低于门槛，未采用。完整输入 JSON 为 **82,583／416,583／834,088 B**；证据见 `../../data/performance-audit-20261014/plan-changes/`。 |
| Galgame 手动分支计划保存 | PERF-091 首次创建全局分支计划时逐章查询 adopted adaptation plan，改为来源章节 ID 的有界批读取；按章节顺序保持完整 parse/hash 和“未采用计划”错误语义。 |
| WebGAL 导出 | PERF-092 复用由同一 `detail.scenes` 构建的 export scene ID Map；删除其后的逐 choice 全场景重复查找，保留 Map 上的缺失目标报错。 |
| 资产依赖图布局候选 | PERF-090（原提交 `4337935c5`）对每轮布局排序预计算邻居 barycenter；1k／5k／10k 节点仅改善 **24.5%／22.2%／24.8%**，均低于门槛，坐标/层级输出相同，未采用。 |
| 资产依赖图后代计数 | PERF-093 将每个节点一次字符串 BTreeSet/队列遍历改为整数索引邻接表和复用的访问代次；输出的精确唯一 `downstream_count` 不变。 |
| 资产依赖图布局排序 | PERF-094 将 `nodeCluster` 对 ID、类型、标题和摘要的拼接、小写及类别判断从每次 comparator 调用收敛为每个可见节点一次；分层与同级排序规则不变。 |
| 根项目漫画目录与无选章空态 | PERF-095 根项目 MangaTab 不再对所有章节读取正文和分析；PERF-096 在未选章时不取档案、立绘和全项目图片，首次选择时正常加载。单章自动选择、固定章节、派生快照及独立漫画路径保留各自需要的读取。 |
| 项目图库图片加载候选 | 在 Chromium 153、生产 Vite、1000 张图片卡片和滚动容器中试过原生 `loading="lazy"`（含图片固有宽高、先设置 loading 再设置 src）；两版页面 load 时都发出 **1000** 个图片请求、响应 **414,000 B**。两轮交错 P50 挂载 **75.3→73.3 ms**（未达 30%），JS heap **8,786,947→8,821,295 B**（候选增加 0.39%），故撤回，不计为优化；证据 `../../data/performance-audit-20261015/image-gallery-lazy-final/report.json`。 |
| 项目图库批量渲染 | PERF-101 将首屏 DOM 限为 500 张并通过按钮追加；1000／2000 张挂载时间下降 **42.22%／66.48%**、GC 后 JS heap 下降 **27.08%／51.81%**。API JSON 和数据库读未变，详见 PERF-101。 |
| 项目图库数据库过滤 | PERF-102 为 `generated_image.project_id` 增加索引；100,000 行、1,000 行命中时数据库执行 P50 下降 **70.01%**，结果顺序不变；完整 JSON 投影和媒体传输未计入，详见 PERF-102。 |
| 章节图库数据库过滤 | PERF-103 为 `generated_image(project_id, chapter_id)` 增加复合索引；100,000 行、100 行命中时数据库执行 P50 下降 **95.49%**，结果顺序不变；详见 PERF-103。 |
| 工作流按章最新运行索引候选 | PERF-104 在 100,000 条运行、1,000 个章节、每章 10 条历史上尝试 `workflow_run(project_id, workflow_kind, chapter_id, updated_at DESC, id DESC)`；按实际完整模型投影 P50 **17.005→12.317 ms（27.57%）**，低于门槛，未修改索引；窄投影的 44.33% 不作为收益。 |
| 工作流项目列表数据库排序 | PERF-105 为 `workflow_run(project_id, updated_at DESC, id DESC)` 增加索引；100,000 行、100 条返回时数据库执行 P50 下降 **96.21%**，结果顺序不变；详见 PERF-105。 |
| 任务历史数据库分页 | PERF-106 为 `task_queue(project_id, created_at DESC)` 增加索引；100,000 行、100 条返回时数据库执行 P50 下降 **97.36%**，结果顺序不变；详见 PERF-106。 |

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

以上不是整站所有性能目标已经完成的声明。首屏交付、核心计算、真实 Host 进程树、1/4/8 并发读请求、LLM TLS/SSE、CLI 写作与恢复链路、千章侧栏、本地及 API 嵌入和大型 JSON 交付已经验证。前端章节目录现使用索引与按需单章详情；完整 `list_chapters` API 合同保留兼容，仍需继续审查其他随项目规模增长的请求数据。

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
- 前端只持有当前章完整内容；切章迟到响应不能覆盖后来选择或写后结果；加载完成前不开启正文编辑/草稿恢复。保存、生成结果核验及优化完成后的刷新可显式指定目标 chapter ID。上一章结尾在展开上下文时读取。Story Events 使用 PERF-019 分析投影；根项目 MangaTab 使用 PERF-095 章节索引及选中详情。
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

### PERF-077：事实反馈摘要的重复章节与全量历史读取

- 请求链路：`FactFeedbackPanel` 仅在反馈工作流运行时每 1.5 秒加载摘要。原 Orchestrator 同时读取完整反馈来源章节、完整项目章节和全部非章节事实变更，随后才过滤无变化行并截断到 100 条。
- 修复：数据库一次读取生成当前来源修订与可用章节投影；事实历史在 PostgreSQL 中按 `project_id, id DESC` 部分索引选取候选 ID，再通过主键读取有限完整行。数据库文本比较只做无漏项的候选筛选，Rust 保留原 `serde_json::Value` 相等判断并以 ID 游标补页，保持最新 100 条、顺序和数字边界行为。
- A/B 使用隔离 PostgreSQL 与相同 `release -O3, LTO=false` 构建，测完整 Orchestrator 摘要方法、权限检查、数据库读取及 DTO 序列化；不包含 HTTP socket 传输。夹具为 1000 章、每章 9 KiB 正文和 8 KiB 分析数据、每项目 10,161 条事实历史，分别测 0／100／1000 条已应用章节来源。15 次交错调用、2 次预热，报告剩余样本中位数；三类 DTO 和响应字节在每次调用间逐值相同。
- P50：**80.77 → 43.89 ms（45.67%）／86.26 → 45.49 ms（47.26%）／93.48 → 53.48 ms（42.80%）**。P95：**91.38 → 50.96 ms（44.23%）／98.91 → 53.59 ms（45.82%）／96.78 → 56.06 ms（42.08%）**。响应 JSON 保持 **108,626–108,627 字节**；基准期间 PostgreSQL 记录到目标索引 31 次扫描，测试断言索引被使用。
- PostgreSQL 与 DTO 等值对照覆盖 `1`／`1.0`、`-0.0`／`0.0`、无变化行、分页补足、来源覆盖及历史顺序。没有减少轮询间隔、增加业务缓存或改变 API。前端本轮无需改动；已核实其轮询只在工作流运行时启用。
- 验证：隔离 PG 手动性能测试通过；`cargo fmt --all -- --check`、`git diff --check` 通过。原始样本与完整测试输出位于 `data/performance-audit-20261013/fact-feedback-summary/`。
- 部署边界：索引迁移沿用当前 PostgreSQL 迁移事务，建索引期间会短暂阻塞该表写入；本基准未测生产表上已有大量历史时的迁移耗时。

### PERF-078：派生媒体工作台的源快照列表

- 请求链路：漫画、视频和 Galgame 派生工作台初载及刷新先取源快照章节。导航/选择器只显示 snapshot ID、源章 ID、章号和标题；漫画/视频详情才需要完整正文，Galgame 的来源引用只显示章号。旧列表接口仍把每章正文和分析 JSON 全部编码后返回。
- 修复：新增 `SourceSnapshotChapterIndex` 和 source-snapshot worker v2 的窄索引、单章详情操作。索引按项目当前 source snapshot 筛选并按章号排序，返回 `id/snapshot_id/source_chapter_id/chapter_number/title`；详情查询同时限定派生项目的 root、当前 snapshot 和 source chapter ID。Worker 对新操作沿用 Viewer 权限校验。Comic/Video 首次选中一章时读取窄索引和当前章，切章时按源章 ID读取正文；项目/读取序号保护迟到响应。Galgame只读取索引。原完整读口保留。
- 隔离 PostgreSQL A/B 使用相同 `release -O3, LTO=false` 构建，1000 章、每章 9 KiB 正文和 8 KiB 分析数据。测数据库投影、完整 DTO 构造与 JSON 序列化，候选包含索引及默认选中章的详情。旧列表 P50/P95 **41.06/47.39 ms**，候选 **3.09/6.02 ms**，分别下降 **92.48%/87.30%**；JSON **17,913,472 → 109,482 字节，减少 99.39%**。每章的 ID、snapshot/source identity、章号、标题相同；候选选中章 DTO 与旧列表相应行逐值相同。
- 计时未包含 Worker HTTP socket 往返；响应载荷数据来自实际 DB 查询与 API DTO 序列化。变化没有使用缓存、预取或降低刷新频率。
- 验证：源快照 DB 回归 **1/1** 通过，手动 A/B **1/1** 通过；Host/Worker/API/DB/runtime/control/process catalog/server/assembly `cargo check --all-targets`、rustfmt、diff 检查通过；API type parity 和 generation schema parity 通过，`a4n-api-contract` 单测 **108/108** 通过。Vitest **138 文件/509 项**、生产构建通过；真实 Chromium media-authoring **7/7** 通过，包含默认章、按需选章、保存/采用计划、Video 项目切换及 Galgame workspace，覆盖 390/1024/1440px。
- 证据：`data/performance-audit-20261013/source-snapshot/`；worker contract capability 更新为 `source-snapshot-read-v2`。

### PERF-079：项目协作摘要成员行聚合

- 请求链路：项目列表最多读取 100 个项目，`ProjectReadWorker` 批量请求 `project_access_info_batch`，该查询原本把项目成员的 `(project_id, user_id, role)` 全部传回 Worker，只为了统计成员数和保留当前用户角色。
- 修复：PostgreSQL 以项目分组执行 `COUNT(*)`，用参数化 `MAX(role) FILTER (WHERE user_id = $1)` 取当前用户角色；权限判定、角色解析、所有者计数和邀请可见范围保持不变。
- 隔离 PostgreSQL 16 使用 100 个项目、每项目 1000 名成员。容器固定 CPU 18/19，通过 loopback TCP 运行 1 组预热及 8 组交错采样。成员查询 P50 **29.540 → 19.399 ms（34.33%）**；CSV 结果 **1,381,300 → 1,492 字节（减少 99.89%）**，行数 **100,000 → 100**。计时只覆盖成员 SQL 与 psql 结果传输/格式化，不外推为完整项目列表 API 改善。
- 同一测量方式下，10,000 行 P50 改善 **28.81%**，低于延迟门槛；500 行增加 **0.022 ms**。因此本项收益只计入高基数场景。单独测试 100,000 条未过期邀请的分组查询只改善 **8.23%**，没有修改邀请统计实现。
- 验证：独立 PostgreSQL 上 `collaboration::tests::collaboration_flow_keeps_reads_pure_and_mutations_explicit` **1/1** 通过，覆盖所有者成员数、有效/过期邀请和成员角色；Rust 格式检查与 diff 检查通过。
- 证据：`data/performance-audit-20261014/collaboration-summary/`，包括合成基准 SQL 和各负载结果。

### PERF-080：视频内联播放重复解码 Base64

- 请求链路：`useVideoPlaybackSource`、Video Assembly 和 Postprocess 在每次选择/重载视频时调用 `videoPlaybackDataToDataUrl`。Host 从文件读取至多 256 MiB，再用 Rust `base64::STANDARD` 编成 API 字符串；前端原先用 `atob` 解码整份内容后丢弃结果，随后 `<video>` 又从同一 data URL 解码一次。
- 修复：保留 MIME 和空载荷校验，移除多余的前端整串 Base64 解码。Host 自己生成该字符串；播放器继续解析原 data URL。下载路径仍通过 `videoPlaybackDataToBlob` 解码实际需要写入的字节。
- Chromium 153 的实际 Vite 页面测试用 64 MiB Base64 输入，交错测量五个样本。helper P50 **124.7 → 0.1 ms**，`performance.memory.usedJSHeapSize` 增量 **67,108,955 → 4,836 字节**。相同浏览器 smoke 确认 `fetch(data URL)` 的媒体类型和字节尾部与原值相同；非法 Base64 交给 `<video>` 后会触发 `error` 事件，播放器现有错误/回退处理仍负责反馈。
- 支持性 Node.js 24.21.0/V8 测量：64 MiB helper P50 **145.710 → 0.340 ms**；128 MiB P50 **291.069 → 0.318 ms**，进程 `VmHWM` **530.2 → 231.8 MiB（下降 56.28%）**。这些数字只覆盖 Base64 helper 阶段，不包含 HTTP 传输或媒体解码。
- 验证：Vitest **138 文件/511 项**、TypeScript/Vite 生产构建通过；Chromium 153 实际打开生产 helper 的 Vite smoke 页通过。`test:browser-video-scrub` 未完成：plain-Web harness 调用了不可用的 Tauri `plugin:event|listen`，首个 `waitUntil` 超时；不记为通过或产品失败。
- 证据：`data/performance-audit-20261014/video-playback-validation/`。

- 邻近下载 Blob 候选也做了 A/B：移除 `.buffer.slice` 仅改善峰值 RSS **13.37%/14.20%**、P50 **9.47%/5.98%**（64/128 MiB），未达门槛；Node 上改用 `fetch(dataURL).blob()` 在 64 MiB 时慢 **4.81×** 且峰值 RSS 高 **92%**，未采用。

### PERF-081：视频上传 Base64 流式落盘

- 请求链路：视频素材、后期插入/替换、参考媒体和画布导入在前端经 `FileReader` 生成 Base64 JSON；Runtime `uploaded_video_source` 原先用 `decode_uploaded_bytes` 生成完整 `Vec<u8>`，再写入应用临时文件。单文件允许至多 **512 MiB**。
- 修复：`a4n-media` 新增共享 `decode_uploaded_bytes_to_writer`，复用相同的空值、Base64 长度、解码错误与解码后大小校验；Runtime 用 64 KiB `DecoderReader`/`BufWriter` 直接写既有临时文件。错误时仍由同一 cleanup guard 删除部分文件；API 和 Base64 上传合同不变。
- 隔离 Rust release A/B 用 64 MiB `base64::STANDARD` 输入，CPU 固定 18/19，交错 12 组并丢弃前三组预热。原路径 P50 **100.975 ms**、`VmHWM` **151.562 MiB**；流式路径 P50 **98.010 ms**、`VmHWM` **87.770 MiB**。时延只改善 **2.94%**，不作为收益；峰值 RSS 下降 **42.09%** 达到门槛。`cmp` 确认两种输出与原始字节完全相同。
- 计时只覆盖 Base64 解码和临时文件写入，不含 HTTP JSON 解析、上传传输及后续 ffprobe/导入；请求仍携带 Base64 字符串，网络与解析成本未在本项中降低。
- 验证：`a4n-media` 流式/缓冲字节和大小边界回归 **1/1** 通过；Runtime 后段 Base64 错误清理部分临时文件 **1/1** 通过；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 证据：`data/performance-audit-20261014/video-upload-streaming/`，含独立 Release A/B harness、输入与等值输出。

### PERF-082：长视频分镜输入的深拷贝和全量重渲染

- 请求链路：用户编辑 `VideoPlanningWorkspace` 中已保存的视频分镜时，每次 textarea 输入都会进入 `VideoSegmentPlanEditor.update`。旧实现深拷贝整份 `segments`，所有分段对象和更新回调都变成新引用，导致全部 `SegmentEditor` 子树重渲染；父级 dirty 检查也重复序列化基线和草稿。
- 修复：编辑从现有只读分段引用开始；每次更改用共享 `updateAtPath` 和函数式 setter 只复制分段数组、目标段和编辑路径上的容器，未改动分段保留原引用。稳定更新回调配合 `React.memo` 只重渲染目标段。编辑期间只序列化一次基线，仍逐次比较草稿内容，因此把字段改回原值会恢复 clean 状态，保存/取消行为不变。
- A/B 在 Chromium **153.0.8010.52** 打开生产 Vite 夹具，Node、ChromeDriver 和 Chromium 固定在 CPU 18/19。每个工作负载含 **8** 次文本输入；每版先跑 2 组预热，再以 5 组样本的 P50 汇总八次 `input` dispatch 与同步 React commit 的累计时间。50／100／200 段计划 JSON 分别为 **55,843／111,805／224,905 字节**：P50 **211.5 → 11.6 ms（94.52%）／459.7 → 17.0 ms（96.30%）／976.9 → 24.2 ms（97.52%）**。
- 完整保存 JSON 的 SHA-256 在前后版本逐负载一致；夹具还断言源计划未被修改、恢复原值会关闭 Save，1440／390 宽度无横向溢出。该计时包含同步事件处理和 React DOM commit，不包含下一帧绘制或真实 API 保存。
- 验证：`npm run test:run -- src/pages/novel-detail/video/VideoPlanningWorkspace.test.tsx` **8/8**；`npm run build` 通过。生产浏览器 A/B 与上述等值、恢复及布局断言通过；Vite 保留已有超过 500 kB 的 chunk 提示。
- 证据：`data/performance-audit-20261014/video-plan-editor/{before-final,after-refactor}/report.json`；运行入口为 `frontend/e2e/performance/check.mjs` 的 `A4N_PERF_CASE=video-planning`。深工作树若触及 Chromium Unix socket 路径长度，可用 `A4N_PERF_CHROME_TMPDIR` 指向 `/home/fufu/Code/A4N/work/` 下的短路径别名；目标临时目录仍须位于该工作树内。

### PERF-083：影片剧本逐字段编辑的全量深拷贝和镜头重渲染

- 请求链路：视频工作区进入 screenplay 后，`ShootingScriptTab` 将当前剧本传给 `StructuredScriptEditor`。每次镜头 textarea 输入都会走 `update(path, value)`；旧实现深拷贝全部 scenes/shots/beats，再更新一个字段。新对象和新回调令所有镜头子树重新渲染。父级 dirty 比较仍保留原有完整值语义，本项基准也包含该比较。
- 修复：视频分镜编辑 PERF-082 共用的 `updateAtPath` 按路径复制剧本、场景、镜头及目标字段容器；更新使用稳定函数式 setter。镜头组件接收稳定的场景/镜头索引并通过 `React.memo` 复用未修改的镜头子树。场景镜头索引数组在剧本编辑器中每次渲染只构造一次。
- A/B 在 Chromium **153.0.8010.52** 打开生产 Vite 页面，Node、ChromeDriver 和 Chromium 固定在 CPU 18/19。每个工作负载含 **8** 次镜头标题输入；每版先跑 2 组预热，再以 5 组样本的 P50 汇总八次 `input` dispatch 与同步 React commit。25／50／100 镜头剧本 JSON 为 **31,364／62,754／125,626 字节**：P50 **183.6 → 34.8 ms（81.05%）／385.8 → 60.7 ms（84.27%）／775.8 → 103.5 ms（86.66%）**。
- 完整剧本保存摘要逐负载相同；夹具检查源数据不变、更新镜头标题及嵌套构图字段、1440／390 宽度无横向溢出。计时覆盖同步输入事件和 React DOM commit，不包含下一帧绘制或后端保存。
- 验证：`npm run build` 通过；`npm run test:run -- src/utils/updateAtPath.test.ts src/pages/novel-detail/video/VideoPlanningWorkspace.test.tsx` **10/10**。生产 Chromium A/B 的等值、源对象不变与布局断言通过；Vite 保留已有大 chunk 提示。
- 证据：`data/performance-audit-20261014/shooting-script/{baseline,after}/report.json`；运行入口为 `frontend/e2e/performance/check.mjs` 的 `A4N_PERF_CASE=shooting-script`。

### PERF-084：Galgame 分支计划编辑的逐章嵌套文档搜索

- 请求链路：用户编辑 Galgame 分支计划时，`MediaAuthoringPanel` 每次输入都会更新 editor document 并重渲染 `GalgameBranchPlanEditor`。该编辑器原先对每个来源章节都在线性扫描整个 `workspace.documents`，寻找首个未过期且已采用的改编计划，复杂度为章节数 × 文档数；随后还要根据适用单元和节拍生成选择项。
- 修复：每次渲染先按来源章节 ID 建 Map，只收录首个有效的已采用改编计划，再线性遍历章节构造原顺序的单元和节拍选项。算法由 O(C×D) 降到 O(C+D)，不保留跨编辑更新的派生缓存，也不改变 API 或计划字段。
- A/B 在 Chromium **153.0.8010.52** 打开生产 Vite 夹具，Node、ChromeDriver 和 Chromium 固定在 CPU 18/19。每版每个负载先跑 2 组预热，再以 5 组样本的 P50 汇总 8 次分支问题输入与同步 React commit。工作区有 100／500／1000 章、每章 4 条改编文档（stale、adopted、draft、重复 adopted），分别为 **97,152／495,152／992,674 字节**。总耗时 P50：**11.8 → 12.3 ms（100 章未达门槛）／30.9 → 11.5 ms（62.78%）／77.2 → 18.1 ms（76.55%）**。
- 保存 JSON SHA-256 前后一致；夹具检查首个有效 adopted 版本仍提供选项，stale/draft/重复版本不改变选择，保存字段正确，1440／390 宽度无横向溢出。计时包含本地 dirty 计算和同步 DOM commit，不包含下一帧绘制或真实保存 API。
- 验证：`npm run build` 通过；生产 Chromium A/B 的首个有效 adopted 选项身份、保存等值与布局断言通过。100 章负载是 **1.5 ms/输入**，比基线慢约 **0.06 ms/输入**；不计为收益。门槛仅由 500／1000 章工作区的 **62.78%／76.55%** 改善满足。
- 证据：`data/performance-audit-20261014/galgame-authoring/{baseline-final,after-final}/report.json`；运行入口为 `frontend/e2e/performance/check.mjs` 的 `A4N_PERF_CASE=galgame-authoring`。

### PERF-085：多章节媒体创作依赖的串行 DB 往返

- 请求链路：Galgame 分支计划覆盖多个来源章节；保存、采用及剧本持久化都会经过 `validate_adopted_authoring_dependencies`。漫画页面同步在存在分页计划依赖时也复用该 helper。旧实现对每个依赖依次执行 project-scoped 主键读取，解析完整 JSON 并验证 canonical SHA-256，再逐条检查 revision/hash/adopted/kind。
- 修复：先拒绝空列表快返及无效 project ID；每批最多取 **64** 个 `(project_id, id)` 记录，建立临时 ID Map。仍按输入引用顺序校验重复、正 ID、缺失、JSON hash、revision/hash、status 和 document kind；每处理一行就从 Map 删除，故完整 JSON 的暂存上限受批大小约束。对外权限、错误分类和首个错误顺序不变。
- A/B 在同一隔离 PostgreSQL 16 schema 使用 **1000** 个真实 chapter/source snapshot chapter 和 adopted 文档，每份文档 JSON 约 **8 KiB**，合计 **8,291,000 B**。旧路径和有界批量路径都逐个完成相同引用字段及 canonical JSON hash 校验；固定 CPU 18/19，5 组交错采样，前 2 组预热，剩余 3 组 P50。与 PERF-086 的 **8,192,000 B** 正文来源负载及 PERF-087 的一条 draft 记录同置后，串行 **237.318 ms**，每批 64 条 **79.038 ms**，下降 **66.7%**；最大批次载荷约 64×8 KiB，即约 **512 KiB** JSON 文档，未将 8.3 MiB 文档集合同时保存在候选校验 Map 中。历史探针的绝对耗时受同 schema 其他文档负载影响，验收采用最新同探针 P50。
- 验证：非 DB 依赖语义回归 **1/1**，覆盖合法多依赖、重复、缺失/项目范围不匹配、非正 project/record ID、revision、draft 状态和错误 kind；实际 Galgame 分支计划到脚本保存集成路径 **1/1** 通过；`cargo check -p a4n-db --all-targets --locked --offline`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- 旁路失败如实保留：`cargo test -p a4n-db --test media_authoring_documents -- --test-threads=1` 4 项中 2 项通过，另 2 项分别在删除 fixture 的 media cleanup 路径和跨项目 source-sync snapshot 设置处失败；失败点早于共享依赖校验。Manga sync 定向测试也因既有 fixture 对自增记录顺序的断言次序不一致失败，该用例依赖列表为空，未进入新 helper。
- 边界：这项 A/B 覆盖本地 PostgreSQL 查询、完整 JSON 取回/解析和 canonical SHA-256 校验，不包括下游模型生成、Runtime HTTP 或 worker 推理。
- 证据：隔离工作树日志 `data/performance-audit-20261014/media-authoring-dependency/recheck-source-final.log`；复现命令：`A4N_TEST_DATABASE_URL=<隔离测试库> cargo test -p a4n-db --lib perf_probe::media_authoring_reference_probe --release -- --ignored --nocapture`。

### PERF-086：Galgame 多章节来源快照的串行读取

- 请求链路：Galgame 分支计划保存时，`run_branch_design_stage` 将每个章节计划中的来源引用合并进分支计划；`save_authoring_document` 在同一事务中逐条读取对应来源快照章节，并检查 root project、snapshot、source chapter、存储哈希和正文 SHA-256。1000 章时旧实现执行 1000 次完整主键读取。
- 修复：来源 ID 每批最多读取 **64** 行；仍按输入顺序逐条执行原身份、重复、缺失和正文哈希校验。独立漫画素材映射检查保留在原位置；该业务路径在输入校验处限定为单来源。
- A/B 使用隔离 PostgreSQL 16、固定 CPU 18/19，在同一 schema 建立 **1000** 个来源快照章节，每条正文 **8 KiB**，合计 **8,192,000 B**。串行与批量路径调用相同的行身份检查和 `source_text_hash`；5 组交错、前 2 组预热、后 3 组计量，最新 P50 **189.626 → 28.061 ms（下降 85.2%）**。候选同时将完整来源行保留上限限制为 64 条。
- 验证：来源身份、哈希、重复和缺失语义单测与 PERF-085 依赖语义单测共 **3/3**；release A/B 探针通过并满足 30% 门槛；`cargo fmt --all -- --check`、`git diff --check` 通过。定向保存集成 `documents_are_project_owned_revision_fenced_and_independent_per_source_snapshot` 在 fixture 的“创作文档基准快照不匹配”断言失败，发生在新来源 helper 前；本次不计为通过。
- 边界：A/B 覆盖来源行读取、身份检查与正文哈希，不包含完整 Galgame 工作流或模型生成时长。
- 证据：`data/performance-audit-20261014/media-authoring-dependency/recheck-source-final.log`；手动探针：`A4N_TEST_DATABASE_URL=<独立测试库> cargo test -p a4n-db --lib perf_probe::media_authoring_reference_probe --release -- --ignored --nocapture`。

### PERF-087：Galgame 千章流程的逐章当前计划与 ID 重读

- 请求链路：Galgame pipeline context 初装时，旧 `load_current_adaptation` 对每章先查 draft，再在没有 draft 时查 adopted；1000 章、其中 1 个 draft 时累计 **1,999** 次查询。分支阶段准备 LLM 输入时又按 1000 个计划 ID 各取一次完整记录。
- 修复：DB 层对来源章节按 draft 优先分两批查询，只为没有 draft 的章节读取 adopted；对分支阶段计划 ID 按 64 条批读。返回未解析行，Orchestrator 仍按原输入顺序逐条解析、选 draft、验证 adopted 状态与 revision，并保留原错误文本；每批完整文档受 64 条上限约束，不保留跨批缓存。
- A/B 在隔离 PostgreSQL 16、CPU 18/19 上测 1000 章、每份 adopted 计划约 **8 KiB** JSON，另为首章加入一个 draft。每版 5 组交错、2 组预热，后 3 组取 P50。context 读取从 **1,037.152 → 95.208 ms（下降 90.8%）**，查询由 1,999 次减为 32 次；分支阶段按 ID 重读 **252.101 → 80.963 ms（下降 67.9%）**，查询由 1000 次减为 16 次。两个路径输出的记录 ID、reference/hash 和 status 序列前后一致。
- 验证：a4n-db authoring 语义单测 **3/3**；同一个隔离 PG release probe **1/1**，同时复测 PERF-085/086；`cargo check -p a4n-orchestrator --all-targets --locked --offline`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- 边界：A/B 计 DB 读取与原始文档解析/哈希，不含 `parse_adaptation` 后续计划投影、LLM 或完整 workflow 耗时。
- 证据：`data/performance-audit-20261014/galgame-document-batches/recheck.log`；手动探针：`A4N_TEST_DATABASE_URL=<隔离测试库> cargo test -p a4n-db --lib perf_probe::media_authoring_reference_probe --release -- --ignored --nocapture`。

### PERF-091：手动分支计划保存的 adopted 依赖串行读取

- 请求链路：`MediaAuthoringPanel` 首次保存无章节范围的 Galgame 分支计划，会让 `authoring_sources(chapter_id=None)` 返回项目所有来源章节；随后 `authoring_inputs_for_sources` 对每章依次查询当前 adopted adaptation plan，并将 reference 按章节顺序写入依赖列表。1000 章时旧实现执行 1000 个串行 project/snapshot/chapter/status 查询，并解析每份约 8 KiB 文档和 canonical hash。
- 修复：多章依赖按来源快照与 chapter ID 每批最多 64 行读取，仅选择 adopted adaptation plan；调用方仍逐章调用 `authoring_source`、解析 canonical 文档并按原次序构建 dependency reference。缺少 adopted plan 时保留原章节号错误。单章读路径不改变。
- A/B 在同一隔离 PostgreSQL 16 schema 使用 PERF-085/086/087 的 1000 个章节和采用计划；每份 document JSON 约 **8 KiB**。逐章 `current_authoring_document(..., Adopted)` 与 64 行批量查询都执行完整 canonical hash 解析并构造有序 ID/revision/hash/status 摘要；固定 CPU 18/19，5 组交错、前 2 组预热、后 3 组 P50 **475.357 → 100.909 ms（下降 78.8%）**，串行查询数 1000→16。输出 reference 摘要完全相同。
- 验证：隔离 release PostgreSQL probe 中 5 条批量 A/B 全通过；authoring 语义单测 3/3；`cargo check -p a4n-orchestrator --all-targets --locked --offline`、`cargo fmt --all -- --check`、`git diff --check` 通过。
- 边界：A/B 计数据库读取、完整文档哈希/解析和 reference 构造；不包括保存 Galgame branch document 的后续事务及 HTTP 往返。
- 证据：`../../data/performance-audit-20261014/manual-branch-save-batch/recheck.log`。

### PERF-092：WebGAL choice 目标的重复场景扫描

- 请求链路：每次 WebGAL 导出先以全部 `detail.scenes` 构建 `scene_file_by_id`，随后 `validate_webgal_choice_contract` 对每个 choice 查该 Map；旧实现命中 Map 后仍从头 `.iter().any(...)` 扫描相同的场景向量。S 个场景、C 个选项时这项重复验证为 O(S×C)，且导出前每次执行。
- 修复：保留 Map 查询和原有“选项指向不存在场”错误，删除 Map 命中后不可达的第二次场景扫描。Map 在同一个 `build_webgal_package` 请求中从 `detail.scenes` 构建，因此两个校验集合始终同源。
- A/B 运行完整生产 `validate_webgal_choice_contract`，构造 1k／5k／10k 个场景、每场 2 个有效 choice，目标集中在后续场景以覆盖线性扫描的高成本位置；每版 5 个样本，warmup 至少 250 ms，CPU 固定 18/19。最新候选 P50 从 **2.394 → 0.288 ms（88.0%）／51.907 → 1.604 ms（96.9%）／202.676 → 4.577 ms（97.7%）**。A/B 两版均对完整有效选择合同返回成功；候选定向测试还确保目标不在导出 scene Map 时仍返回原有错误。
- 验证：baseline/candidate release `webgal_choice_validation_probe` 各 1/1；候选 WebGAL export 定向单测 **5/5**，包含无效目标错误回归；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 边界：测量选择合同校验 CPU，不含读取剧本、媒体文件打包、Base64 或 zip 输出。
- 证据：`../../data/performance-audit-20261014/webgal-choice-validation/report.json`；基线提交为 `83759a956`。

### PERF-093：资产依赖图的字符串后代遍历

- 请求链路：`asset_dependency_graph` 读取项目族记录后构建 `GraphBuilder`，每次依赖图请求调用一次 `GraphBuilder::finish`。旧 `downstream_counts` 对每个节点重新遍历全部可达节点，并用 `BTreeSet<String>` 和 `VecDeque<String>` 复制、排序与去重节点 ID；后代数会进入 API 输出，因此需要保留精确的唯一可达节点数。
- 修复：为图节点建立一次 ID→整数索引，构建整数邻接表；每个源节点遍历时复用一个队列，并用访问代次数组去重。重复关系边先映射为唯一相邻节点；菱形图回归确保共享叶节点只计一次。辅助内存为 O(V+E)，没有全图可达矩阵。
- A/B 固定 CPU 18/19，Rust release；8 层 DAG、边数 2.625×节点数，生成图输入在计时外；每种规模至少 250 ms 预热后取 5 次 P50。1000／5000／10000 节点（2625／13125／26250 边）P50 **94.467→5.335 ms（94.35%）／1155.143→37.748 ms（96.73%）／2735.239→84.155 ms（96.92%）**。各规模排序节点 ID、精确 `downstream_count`、可变标记、变更策略及边摘要均与基线一致。
- 验证：release 基准探针 **1/1**；资产依赖图 builder 语义测试 **2/2**，覆盖重复关系和菱形图唯一后代；`cargo fmt --all -- --check`、`git diff --check` 通过。
- 边界：计时覆盖 `GraphBuilder::finish`，含边投影、后代数、变更策略、环检测和层输出；不含 PostgreSQL 读取与 JSON 序列化，因此这些数据是请求 CPU 阶段耗时，不代表完整 HTTP 延迟。
- 证据：`../../data/performance-audit-20261015/asset-graph-builder/report.json`；基线提交为 `93cfe476f`。

### PERF-094：资产依赖图排序中的重复节点分类

- 请求链路：打开或刷新资产依赖图时，`AssetDependencyGraphDialog` 对新图调用一次 `buildLayout`。`topologyDepthLayers` 的堆排序和 `orderLayerNodes` 的前后扫描排序会多次比较同一批节点；旧比较器每次对两个节点重新拼接 ID、类型、标题、摘要并转小写，再做多次字符串分类检查。
- 修复：每次 `buildLayout` 对可见节点计算一次 `nodeCluster`，后续比较器复用该布局内的聚类排序键。堆优先级、邻居 barycenter、排序轮次与同级比较次序保持原规则；没有跨请求状态。
- A/B 使用 production Vite 构建和 Chromium **153.0.8010.52**，固定 CPU 18/19；8 层 DAG、每个非末层节点连向下一层 3 个节点，平均 2.625 条边/节点。输入为 1000／5000／10000 节点，JSON **808,670／4,079,920／8,183,797 B**；每种规模至少 250 ms 预热后取 5 次 P50：**53.400→25.700 ms（51.87%）／345.600→172.600 ms（50.06%）／781.700→403.100 ms（48.43%）**。三种规模节点位置、层次、画布尺寸和层摘要 SHA-256 均相同。
- 验证：生产 `npm run build` 通过；`npm run test:run -- src/pages/novel-detail/asset-graph/graphCanvas.test.ts` **4/4**；baseline/candidate Chromium 探针各 3 种负载通过；基准脚本 `node --check`、`git diff --check` 通过。
- 边界：基准测 `buildLayout` CPU，不含项目族 DB 读取、React 对话框提交或 Canvas 绘制；不把这项阶段收益外推为完整页面响应时间。
- 证据：`../../data/performance-audit-20261015/asset-graph-layout/{baseline,candidate}/report.json`；基线提交为 `8d480480c`。

### PERF-095：根项目漫画页的全章正文初载

- 请求链路：NovelDetail 根项目漫画页在多章项目初载时只显示章节侧栏且不默认选中章节，但旧逻辑仍通过 `listChapters` 将每章正文和分析都读取、传输并映射到 `ChapterRow`。派生漫画项目已用快照索引，因此此项只处理根项目路径。
- 修复：根项目目录改用已存在的 `listChapterIndex`；用户选中章节后通过现有 `getChapter` 读取正文和分析。单章自动选中、已选章节刷新、迟到响应保护、章节状态和 `list_chapters` API 合同均保留。目录读口只承载 UI 显示和流水线状态所需字段。
- 量化：复用 PERF-017 同一个 1000 章真实列表 A/B：`list_chapters` **11,601,686 B** → `list_chapter_index` **742,686 B**，响应体减少 **93.60%**。此处量化的是 MangaTab 首次目录请求的列表阶段；多章初载不会请求单章详情。
- 验证：索引行和单章详情投影测试包含在漫画工具与 Chapter API 定向测试中，**13/13** 通过；生产 `npm run build` 通过（保留既有大 chunk 提示）。
- 边界：未单独重测整个 MangaTab 页面的端到端耗时，复用 PERF-017 的列表 API 数据对照。单章根项目仍保留自动选择，会在目录后读一次该章详情。
- 证据：`PERF-017` 的千章 `list_chapters`／`list_chapter_index` A/B 记录及 `b08/browser-final-02`；页面来源为 `MangaTab.loadChapters`。

### PERF-096：根项目漫画页未选章时延迟读取资产

- 请求链路：根项目 MangaTab 有多章且没有选中章节时只显示章节侧栏和空态；详情面板不渲染档案、立绘或图库引用，但旧挂载 effect 仍并行调用 list_profiles、list_portraits_by_project、list_images_by_project，并将完整结果保留在组件状态。
- 修复：只在根项目 MangaTab 确认有当前章节后启动这三项读取。多章初载的首次选择从 openChapterDetail 开始读取；单章自动选择在 loadChapters 确认唯一章节后、并行读取详情时开始。固定章节、派生快照、独立漫画及已有刷新/重试行为保留；没有加预取或业务缓存。
- A/B：真实 Chromium 153、生产 Vite 组件、4× CPU 与 4 Mbps/50 ms 网络；1000 章目录、50 个角色档案、200 张立绘，三轮交错。图片记录使用 GeneratedImage 合同形状的 DTO，提示词取自项目 prompt 文本。无选章目录就绪 P50：5000 张 2236.1→2074.8 ms（7.21%），10000 张 2253.9→2073.4 ms（8.01%），均未达到 30% 时延门槛。
- 同一 A/B 的 GC 后 Chromium JS heap P50：5000 张图片 14,758,372→9,008,896 B（38.96%）；10000 张 19,027,196→9,008,704 B（52.65%）。解压 API 响应体（含章节目录与状态）分别为 5,722,957→969,812 B（83.05%）和 10,274,345→969,812 B（90.56%）。
- 行为检查：多章无选章时三项资产 API 均为 0 次；首次选章后各调用 1 次，前后两版选章后的页面文本 SHA-256 相同。单章自动选择专项确认两版均选中第 1 章并各读取资产一次。1000 图片单次样本的 JS heap 下降 19.42%，低于门槛；保留的目标场景是至少 5000 张图片的高规模项目。
- 边界：浏览器加载、JSON 解析、React 状态和 JS heap 使用确定性的合同形状夹具；未运行真实 PostgreSQL/Control，因此不把这些数据解释成数据库查询延迟。目录就绪时间改善低于门槛，没有宣称页面响应提升超过 30%。
- 验证：基线提交 7bb6866c2 与候选生产包的三轮浏览器对照通过；单章自动选择专项对照通过。候选 TypeScript/Vite build、基准脚本 node --check 与 git diff --check 结果见本轮任务记录。
- 证据：../../data/performance-audit-20261015/manga-assets/report.json 与 ../../data/performance-audit-20261015/manga-assets-single/report.json。复现时从 frontend/ 执行：

~~~bash
flock -x /home/fufu/Code/A4N/work/.agent-build.lock env \
  -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy -u NODE_USE_ENV_PROXY \
  NO_PROXY=localhost,127.0.0.1 A4N_PERF_ARTIFACTS=../data/performance-audit-20261015/manga-assets \
  node e2e/performance/manga-assets.mjs
~~~

### PERF-097：漫画参考图高规模候选搜索

- 请求链路：MangaPagePromptList 渲染参考绑定器时，每个角色都把全部角色立绘和项目图片变成 ThemedSelect 选项；打开 ThemedSelect 时再把所有选项投影并渲染到 portal。候选总数为 `立绘 + 项目图片 + 空选项`，每个绑定重复一次。
- 修复：候选不超过 100 项时保留旧下拉。超过 100 项后，空查询不创建候选对象；输入至少两个字符后按既有角色/提示词/页码/图片 ID/路径/metadata 字段搜索，只保留前 100 个可见候选。候选仍按原来的相关度、收藏优先级和标签顺序排序。下拉项显示图片 ID、面板 ID 和短提示词，选中后显示缩略图与身份，避免大图库中无法辨认候选。搜索状态按面板生命周期管理，切换提示页会重置；没有增加服务端 API、缓存或预取。
- A/B：基线 `faab71131` 对候选生产 Vite 构建，Chromium **153.0.8010.52**，3 轮交错；使用 50 个角色、200 张立绘、3 个绑定，项目图片分别为 5000 和 10000 项。宽泛查询 `page` 匹配所有合成项目图片的面板 ID。P50：初次挂载 **59.6→9.5 ms（84.06%）／98.8→8.3 ms（91.60%）**；宽泛搜索 **26.8／45.7 ms**，旧版直接打开完整选择器与新版搜索后打开之和 **329.1→99.6 ms（69.74%）／576.0→114.7 ms（80.09%）**。JS heap（挂载后 GC）**8,614,025→5,004,870 B（41.90%）／13,737,522→6,624,898 B（51.78%）**。
- 行为检查：指针和键盘都能选中搜索结果并启用“添加”，且选择前出现缩略图；候选上限为 100 个可见结果，结果总数与“显示前 100 项”提示一致。1440／1024／390px 页面横向溢出均为 0，已保存默认、搜索和选中态截图。
- 验证：漫画提示模型 Vitest **2/2**；包含漫画与时间线两个目标文件的 Vitest **8/8**；`npm run build`、基准脚本 `node --check`、`git diff --check` 通过。构建保留既有 >500 kB chunk 警告。
- 边界：基准使用符合 API 形状的合成 DTO 与本地 SVG data URL，不包含真实 Control/数据库读取或媒体服务；它量化的是候选构造、React 渲染和浏览器选择器开销，不推断图库 API 或真实图片传输耗时。100 项以下保留旧交互，因此不计为性能收益场景。
- 证据：`../../data/performance-audit-20261015/manga-references-final/report.json` 和该目录下 `screenshots/`。从 `frontend/` 复现：

~~~bash
A4N_PERF_QUERY=page A4N_PERF_ARTIFACTS=../data/performance-audit-20261015/manga-references-final \
  node e2e/performance/manga-references.mjs
~~~

### PERF-098：拆分镜头弹窗的重复时间点扫描

- 请求链路：拆分镜头弹窗中的位置选择器在每次打开或输入变化时重新渲染。旧过滤器对每个时间点都重新查找当前镜头段，并调用 `segmentTimes` 两次线性查找起止点；点数与段数增长时产生嵌套扫描。
- 修复：新增 `pointsInsideSegment`，先查找目标段一次、计算起止时间一次，再用原来的严格区间条件过滤时间点。拆分可选项的 ID、显示顺序、边界排除和缺失引用错误保持原行为；不增加 memo/cache。
- A/B：Chromium **153.0.8010.52** 生产 Vite 包；1000／5000 个时间点及段，每个目标段包含中间一半时间点；2 轮预热、7 轮交错，直接测量完整选项投影并逐项校验结果。P50：1000 项 **10.5→≤0.1 ms**；5000 项 **78.6→0.1 ms（99.87%）**。候选计时受 `performance.now()` 精度限制，报告 0.0 ms 的样本按 **≤0.1 ms** 表述。
- 验证：timeline 定向 Vitest **6/6**（包含 PERF-099 边界检查）；基准脚本 `node --check`、`git diff --check` 和完整前端生产构建通过。
- 边界：A/B 量化的是弹窗位置选项投影函数，不计 Modal/React 全组件或浏览器绘制时间；5000 项是合成压力场景，不代表常规计划规模。减少投影的 JS 主线程 CPU 时间达到门槛，因此保留局部根因修复。
- 证据：`../../data/performance-audit-20261015/timeline-split/report.json`。从 `frontend/` 复现：

~~~bash
A4N_PERF_POINT_COUNTS=1000,5000 A4N_PERF_ARTIFACTS=../data/performance-audit-20261015/timeline-split \
  node e2e/performance/timeline-split.mjs
~~~

### PERF-099：时间点到所属分镜的嵌套查找

- 请求链路：拆分弹窗无当前段选择时，编辑器源投影按播放时间确定所属段；原 `segmentAt` 对每个候选段调用 `segmentTimes`，后者再从全部点中线性查找两端点。命中越靠后，段数和点数的乘积会进入同步主线程路径。
- 修复：一次建立时间点 ID→时间 Map（重复 ID 保留原 `.find` 的第一个结果），仍按原分段顺序扫描；命中首个时间区间即返回。缺失时间点继续抛出相同错误，时长末端仍回落到最后一段。
- A/B：同 PERF-098 的 Chromium 生产浏览器基准；查询时间落在第 90% 处的分段，2 轮预热＋7 轮交错。1000 点/段 P50 **4.8→≤0.1 ms**；5000 点/段 **32.6→0.2 ms（99.39%）**；命中的 segment ID 前后相同。
- 验证：timeline 定向 Vitest **6/6**，含重叠区间首匹配与缺失端点错误；共用 `timeline-split.mjs` 报告含投影及查找 A/B。完整前端构建通过。
- 边界：仅测所属段定位函数，不计编辑器其余工作；合成查询落在靠后区间用于覆盖最坏扫描路径。
- 证据及复现入口：`../../data/performance-audit-20261015/timeline-split/report.json`，见 PERF-098 命令。

### PERF-100：项目图库原生 lazy-loading 候选（未采用）

- 请求链路：项目图库把所有图片卡片放在滚动容器中；尝试让浏览器延迟请求屏外图片，以减少初载媒体流量。
- A/B：Chromium 153、生产 Vite、1000 张卡片、两轮交错。原生 `loading="lazy"`（含先设置 loading、补图片固有宽高）仍请求全部 **1000** 张，传输仍为 **414,000 B**；挂载 P50 **75.3→73.3 ms**，低于 30% 门槛；GC 后 JS heap **8,786,947→8,821,295 B**，增加 **0.39%**。
- 决策：没有保留候选改动；根因在当前滚动容器里的图片卡片都仍被挂载，浏览器原生懒加载没有把请求限制到首屏。
- 证据：`../../data/performance-audit-20261015/image-gallery-lazy-final/report.json`。

### PERF-101：项目图库首屏卡片和图片请求过多

- 请求链路：打开 NovelDetail 项目图片图库时，`ImageGalleryPanel` 会按输入数量为每张图片创建卡片和 `<img>`；DOM、React 协调、布局和媒体请求随图库规模增长。PERF-100 已确认此滚动容器中的 `loading="lazy"` 不能阻止全部图片请求。
- 修复：项目图和章节分组图首次都最多渲染 500 张，按钮每次再追加最多 500 张。项目原顺序和分组顺序不变；切换章节范围或收藏过滤重置到首批，切章也会重置。所有图片仍在原始列表中，按钮可继续展开直到全量可见。
- A/B：基线 `56cb77fe2`，Chromium **153.0.8010.52 Arch Linux**，生产 Vite，固定 CPU **18/19**，1000／2000 张合成图片、每档 3 轮交错。首屏挂载 P50 分别 **192.1→111.0 ms（42.22%）／339.8→113.9 ms（66.48%）**；首次渲染后 GC heap 分别 **7,980,665→5,819,781 B（27.08%）／12,470,909→6,009,801 B（51.81%）**。首屏图片请求 **1000→500／2000→500**，传输 **414,000→207,000 B／828,000→207,000 B**。完整展开以及键盘 Enter 显示第 1000 张图片均通过。
- 界面检查：1440／1024／390px 的横向溢出均为 **0**；三种宽度首屏都显示“已显示 500/1,000 张图片”，Enter 将卡片数从 500 增至 1000，末张文本为 `Image 1000`。独立截图评审未发现可见问题。
- 验证：图库 Vitest **2/2**、`npm run build`、性能脚本 `node --check`、A/B 生产 Chromium 脚本及 `git diff --check` 通过。生产构建保留既有 >500 kB chunk 警告。
- 边界：本项减少客户端卡片挂载、浏览器媒体请求、传输及首屏 JS heap；API 仍传入完整 `allImages`，过滤、分组和排序也仍处理完整列表，因此没有测量或宣称 API JSON／数据库延迟收益。图片传输使用每张 114 B 的本地 SVG 夹具，真实媒体字节降幅取决于图片大小；挂载和 heap 结果是生产组件在该浏览器与 CPU 下的合成规模对照。
- 证据：`../../data/performance-audit-20261015/image-gallery-batches/report.json`。从 `frontend/` 复现：

~~~bash
taskset -c 18,19 env A4N_PERF_ARTIFACTS=../data/performance-audit-20261015/image-gallery-batches \
  node e2e/performance/image-gallery.mjs
~~~

### PERF-102：项目图库查询缺少项目过滤索引

- 请求链路：`ImagesTab` 的项目图库调用 `list_images_by_project`；Orchestrator 最终执行 `generated_image` 的 `WHERE project_id = ? ORDER BY created_at DESC`，旧表没有 `project_id` 索引，因此 PostgreSQL 需要扫描整张图片表再排序。该查询返回完整图片 DTO，数据库过滤是请求链路的固定前置成本。
- 修复：新增 `generated_image_project_index` schema migration，创建 `idx_generated_image_project_id` 单列索引；不改变查询、返回字段、排序、权限或错误行为。迁移注册在现有最后一项 `creator_notifications` 之后，并更新运行时 schema gate。
- A/B：隔离 PostgreSQL 16，100,000 行合成 `generated_image` 形状数据，100 个项目、目标项目 1,000 行，5 轮交错；同一查询用 `EXPLAIN (ANALYZE, BUFFERS)` 测量。无索引 P50 **14.223 ms**（`Seq Scan` + `Sort`），有索引 **4.265 ms**（`Bitmap Index Scan` + `Bitmap Heap Scan` + `Sort`），下降 **70.01%**；shared hit blocks 由 **7,146** 降至约 **1,006–1,009**，索引大小 **745,472 B**。每轮返回的 1,000 个 ID SHA-256 顺序相同。
- 验证：`a4n-migration` 与 `a4n-db-models` `cargo check --locked --offline` 通过；迁移 SQL 单测 **1/1**；隔离 PostgreSQL 的 `a4n-db` 源快照集成测试 **1/1** 通过（覆盖全量迁移初始化）；基准脚本 `node --check` 和 `git diff --check` 通过。
- 边界：A/B 测的是数据库执行时间，不是完整 HTTP/JSON/媒体传输延迟；合成数据每项目命中率为 1%，真实命中率越高索引收益越低，需在生产 `EXPLAIN` 复核。索引迁移使用常规事务型 `CREATE INDEX`，大表部署期间可能短暂阻塞图片表写入；若生产表规模显著更大，应安排维护窗口后再执行。
- 证据：`../../data/performance-audit-20261015/generated-image-index/report.json`。从仓库根目录的工作树执行：

~~~bash
A4N_TEST_POSTGRES_IMAGE=docker.m.daocloud.io/library/postgres:16 \
  python3 scripts/tests/run_with_test_database.py \
  --report-dir data/test-runs/perf-image-index-20261015 \
  --timeout 60 -- node scripts/performance/generated-image-index.mjs
~~~

### PERF-103：章节图库查询缺少项目和章节复合索引

- 请求链路：切换 NovelDetail 章节图库时，`ImagesTab` 调用 `list_images_by_chapter`；Orchestrator 执行 `generated_image` 的 `WHERE project_id = ? AND chapter_id = ? ORDER BY created_at DESC`。PERF-102 的单列项目索引仍需在目标项目的全部图片中筛选章节，章节规模越小浪费越明显。
- 修复：新增 `generated_image_project_chapter_index` schema migration，创建 `idx_generated_image_project_chapter` 复合索引；不改变查询、返回字段、排序、权限或错误行为。迁移注册在 PERF-102 索引之后，并更新运行时 schema gate。
- A/B：隔离 PostgreSQL 16，100,000 行合成 `generated_image` 形状数据，100 个项目、每项目 10 个章节、目标项目/章节 100 行，5 轮交错；同一查询用 `EXPLAIN (ANALYZE, BUFFERS)` 测量。无索引 P50 **17.994 ms**（`Seq Scan` + `Sort`），有索引 **0.812 ms**（`Bitmap Index Scan` + `Bitmap Heap Scan` + `Sort`），下降 **95.49%**；索引大小 **761,856 B**。每轮返回的 100 个 ID SHA-256 顺序相同。
- 验证：`a4n-migration` 与 `a4n-db-models` `cargo check --locked --offline`、迁移顺序测试和迁移 SQL 单测通过；隔离 PostgreSQL 的 `a4n-db` 源快照集成测试仍通过；基准脚本 `node --check` 和 `git diff --check` 通过。
- 边界：A/B 测的是数据库执行时间，不是完整 HTTP/JSON/媒体传输延迟；真实章节命中率越高索引收益越低，需在生产 `EXPLAIN` 复核。索引迁移使用常规事务型 `CREATE INDEX`，大表部署期间可能短暂阻塞图片表写入，应安排维护窗口。
- 证据：`../../data/performance-audit-20261015/generated-image-chapter-index/report.json`。从工作树根目录执行：

~~~bash
A4N_PERF_IMAGE_QUERY=chapter \
  A4N_PERF_ARTIFACTS=data/performance-audit-20261015/generated-image-chapter-index \
  A4N_TEST_POSTGRES_IMAGE=docker.m.daocloud.io/library/postgres:16 \
  python3 scripts/tests/run_with_test_database.py \
  --report-dir data/test-runs/perf-image-chapter-index-20261015 \
  --timeout 60 -- node scripts/performance/generated-image-index.mjs
~~~

### PERF-104：工作流按章最新运行复合索引候选（未采用）

- 请求链路：`latest_workflow_runs_for_chapters` 按项目、工作流类型和章节 ID 集合读取历史运行，再用 `DISTINCT ON (chapter_id)` 选每章最新记录；这是恢复和章节摘要路径的批量读模型。
- 候选：增加 `(project_id, workflow_kind, chapter_id, updated_at DESC, id DESC)` 复合索引，目标是减少全表扫描和排序。
- A/B：隔离 PostgreSQL 16，100,000 条运行记录、目标项目 20,000 条、`video` 类型 10,000 条、1,000 个章节，每章 10 条历史；5 轮交错，查询选择完整运行行并执行 `DISTINCT ON`。无索引 P50 **17.005 ms**（`Seq Scan` + `Sort` + `Unique`），有索引 **12.317 ms**（`Bitmap Index/Heap Scan` + `Sort` + `Unique`），仅下降 **27.57%**；结果 1,000 个章节 ID 顺序完全一致。窄选列的早期测量为 44.33%，与实际完整模型投影不符，不作为验收依据。
- 决策：撤回候选索引和夹具，不改变产品 schema；完整行投影受 heap 读取和排序主导，当前索引复杂度与写入成本不值得引入。
- 证据：`data/test-runs/workflow-run-index-20261015-v3/command.log`；本候选没有产品提交。

### PERF-105：工作流项目列表缺少排序索引

- 请求链路：`list_workflow_run_models` 在项目列表没有额外状态/类型筛选时执行 `WHERE project_id = ? ORDER BY updated_at DESC, id DESC LIMIT 100`。已有 `workflow_run(project_id, status)` 索引只能筛选项目，数据库仍需读取项目历史并排序后取前 100 条。
- 修复：新增 `workflow_run_project_updated_index` schema migration，创建 `idx_workflow_run_project_updated`（`project_id, updated_at DESC, id DESC`）；不改变列表限制、完整运行 DTO、排序、权限或错误行为，并更新运行时 schema gate。
- A/B：隔离 PostgreSQL 16，100,000 条完整形状运行记录，目标项目 20,000 条，保留已有 `(project_id, status)` 索引，5 轮交错；无候选索引 P50 **7.702 ms**（`Limit` + `Bitmap Heap Scan` + `Sort`），有候选索引 **0.292 ms**（`Limit` + `Index Scan`），下降 **96.21%**。索引大小 **4,079,616 B**，每轮返回 100 个 ID 顺序完全一致。
- 验证：migration SQL 单测、迁移顺序测试、`a4n-migration`/`a4n-db-models` check、隔离 PostgreSQL 全迁移初始化集成测试、基准脚本 `node --check` 和 `git diff --check` 通过。
- 边界：A/B 测量数据库执行时间，不包含 JSON/HTTP；仅覆盖项目列表默认 100 条路径，状态/类型筛选仍由现有索引和查询计划决定。索引迁移使用事务型 `CREATE INDEX`，大表部署需安排维护窗口。
- 证据：`../../data/performance-audit-20261015/workflow-run-list-index/report.json`。从工作树根目录执行：

~~~bash
A4N_PERF_ARTIFACTS=data/performance-audit-20261015/workflow-run-list-index \
  A4N_TEST_POSTGRES_IMAGE=docker.m.daocloud.io/library/postgres:16 \
  python3 scripts/tests/run_with_test_database.py \
  --report-dir data/test-runs/workflow-list-index-20261015 \
  --timeout 60 -- node scripts/performance/workflow-run-list-index.mjs
~~~

### PERF-106：任务历史分页缺少项目时间索引

- 请求链路：`persisted_task_history` 在项目任务历史页执行 `WHERE project_id = ? ORDER BY created_at DESC OFFSET/LIMIT`，旧表没有项目/时间复合索引，数据库需扫描项目任务后排序。
- 修复：新增 `task_queue_project_created_index` schema migration，创建 `idx_task_queue_project_created`（`project_id, created_at DESC`）；不改变分页参数、完整任务 DTO、排序、权限或错误行为，并更新运行时 schema gate。
- A/B：隔离 PostgreSQL 16，100,000 条完整形状任务记录，目标项目 20,000 条，默认返回 100 条，5 轮交错。无索引 P50 **15.588 ms**（`Limit` + `Seq Scan` + `Sort`），有索引 **0.412 ms**（`Limit` + `Index Scan`），下降 **97.36%**；索引大小 **3,178,496 B**，每轮返回 100 个 ID 顺序完全一致。
- 验证：migration SQL 单测、迁移顺序测试、`a4n-migration`/`a4n-db-models` check、隔离 PostgreSQL 全迁移初始化集成测试、基准脚本 `node --check` 和 `git diff --check` 通过。
- 边界：A/B 测量数据库执行时间，不包含 JSON/HTTP；只覆盖带项目过滤的历史页，未传项目 ID 的全局历史仍由排序计划决定。索引迁移使用事务型 `CREATE INDEX`，大表部署需安排维护窗口。
- 证据：`../../data/performance-audit-20261015/task-history-index/report.json`。从工作树根目录执行：

~~~bash
A4N_PERF_ARTIFACTS=data/performance-audit-20261015/task-history-index \
  A4N_TEST_POSTGRES_IMAGE=docker.m.daocloud.io/library/postgres:16 \
  python3 scripts/tests/run_with_test_database.py \
  --report-dir data/test-runs/task-history-index-20261015 \
  --timeout 60 -- node scripts/performance/task-history-index.mjs
~~~

### PERF-088：ChapterVisualPlan 编辑重复序列化与本地草稿写入

- 请求链路：`useDirectorPlan.updateFields` 在每次输入时更新当前 plan；原实现为 dirty 比较序列化当前 plan，`replace` 再分别序列化旧/新 plan 判断 revision，立即本地备份再序列化并写入一次，随后 effect 又序列化并写入相同的 plan。章节计划越大，每次按键做的完整 JSON 遍历和同步存储越多。
- 修复：保存当前 plan 对应的 JSON 字符串，并让 dirty、editor revision、save、beforeunload 和本地备份复用它。`updateFields` 仍同步写 sessionStorage，effect 仅在该 plan 对象还没有同步备份时写入；存储失败时不标记完成，因此 effect 仍会重试。其他替换路径仍由 effect 备份。
- A/B 在真实 Chromium **153.0.8010.52** 的 production Vite fixture 中运行 `useDirectorPlan`、React commit 和同步 sessionStorage；CPU 固定 18/19，每个负载先 2 组预热，再测 5 组，每组 8 次镜头字段编辑。100／500／1000 段计划为 **82,583／416,583／834,088 B**。P50：**16.8 → 9.7 ms（42.3%）／114.0 → 56.4 ms（50.5%）／234.2 → 112.3 ms（52.1%）**。
- 迭代记录：仅复用序列化字符串、仍让 effect 重复写同一备份时，1000 段 P50 为 **168.4 ms（28.1%）**，未达门槛；添加同对象备份去重后降至 **112.3 ms（52.1%）**。未采用低于门槛的中间实现。
- 未采用的相邻候选 PERF-089：在 `planChanges` 中对 before/after 引用相同的点和镜头跳过 JSON 深比较；100／500／1000 段分别下降 **22.0%／21.0%／18.7%**，均低于验收门槛，因此没有改动该实现。
- 验证：`npm run build` 通过（保留既有大 chunk 提示）；`npm run test:run -- --maxWorkers=2` 为 **139 文件/513 项通过**；真实浏览器 `node e2e/director-timeline/check.mjs` **18 项通过**，覆盖保存期间继续编辑、SSE 回执、冲突合并/撤销、章节切换、桌面与窄屏布局；director-plan A/B 的 dirty/revert、sessionStorage 内容、editor revision 与布局断言通过。
- 边界：该 A/B 测量 hook 与同步编辑提交，不包括后端保存请求耗时或完整章节视觉计划页面的所有组件。
- 证据：`../../data/performance-audit-20261014/director-plan/{baseline/report.json,after-final/report.json,serialization-only-intermediate-report.json,ui-regression-report.json}`。复现入口为 `frontend/e2e/performance/check.mjs` 的 `A4N_PERF_CASE=director-plan`；基线提交为 `b25e60445`。profile 使用 `A4N_PERF_TEMP_ROOT=../.tmp/director-plan-profile` 和 `A4N_PERF_CHROME_TMPDIR=../.tmp/director-plan-chrome`，将 Chromium 临时 socket 放在短路径下；先设 `A4N_PERF_BASELINE_REF=b25e60445` 生成 baseline，再去掉该变量生成候选。

### PERF-090：资产依赖图布局 barycenter 候选（未采用）

- 请求链路：打开项目族资产依赖图时，`buildLayout` 需要对分层节点反复排序；5 轮前后扫描的 comparator 都重新遍历相邻边、分配位置数组并计算邻居 barycenter。每轮排序使用的邻居顺序快照固定，因此尝试每轮先批量计算 barycenter，排序时只读 Map。
- A/B 在生产 Vite + Chromium **153.0.8010.52**、固定 CPU 18/19，以 8 层 DAG、每节点约 2–3 条跨层边测量完整 `buildLayout`。输入为 1000／5000／10000 节点、581,189／2,945,188／5,912,434 B，分别有 2,625／13,125／26,250 条边；5 组样本 P50 **40.4→30.5 ms（24.5%）／283.5→220.6 ms（22.2%）／616.2→463.4 ms（24.8%）**。三种规模的全部节点坐标、层级、画布尺寸与层摘要逐项完全相同。
- 决策：全规模均低于 30% 目标，候选代码和夹具未保留。
- 证据：`../../data/performance-audit-20261014/asset-graph/{baseline/report.json,after/report.json}`；baseline 源码来自 `4337935c5`。
