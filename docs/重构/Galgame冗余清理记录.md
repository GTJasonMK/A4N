# Galgame 功能 BFS 冗余清理记录

日期：2026-08-11
编号范围：R-1261～R-1337

## 搜索起点与覆盖范围

以“用户在 Galgame 页查看剧本、运行/预览/恢复/取消管线、编辑行与分支、规划并生成视觉资产、绑定媒体、导出 WebGAL，以及源文本变化后的场重建”为功能起点，按广度优先顺序检查：

1. 前端入口：`GalgameTab` 的剧本读取、管线控制、行/场/选项编辑、CG 指令、资产规划与生成、媒体绑定和 WebGAL 导出。
2. Host 合同：`a4n-api-contract`、生成 Host 类型、command manifest、Tauri 注册、Web dispatch 与 CLI capability 元数据。
3. Runtime：`actions/galgame_records` 查询、mutation、pipeline、asset generation、media、export 及 Runtime state/facade。
4. Orchestrator：权限 application、pipeline JSON/prompt/run/state/persist、read model、source-sync 决策、资产规划/生成、媒体、项目族导入导出。
5. DB：Galgame script/scene/line/choice、generation spec、media binding/provenance、项目进度聚合、source-sync 清理和项目族 ID remap。
6. 隐式依赖：Serde/Schemars、生成 TypeScript、prompt registry、workflow checkpoint/business key、PostgreSQL baseline、历史 migration、deferred trigger、reverse-integrity trigger 与运行时 schema marker。
7. 横向对比：全仓搜索同名 DTO、stage 枚举、presentation 解析器、CG 别名、importance 默认值、pose 可选分支、`source_line_ids` 解析器和重复测试。
8. 框架检查：未发现 Galgame 相关 XML、反射入口、动态代理、AOP 切面、依赖注入容器注册或额外路由表；动态入口由 command manifest、Tauri/Web registry、prompt registry 和 workflow key 覆盖。

## 活跃调用拓扑

```text
GalgameTab
  -> generated Host API
  -> Tauri command / Web dispatch / CLI capability
  -> Runtime transport action
  -> Orchestrator user-validated application
  -> Galgame pipeline/read-model/mutation/asset/media owner
  -> a4n-db records and PostgreSQL integrity triggers

pipeline start/preview
  -> shared typed start stage and workflow identity
  -> shooting projection
  -> branch design
  -> script adaptation
  -> character staging
  -> visual planning
  -> typed complete presentation document
  -> transactional script persistence
  -> generation-spec planning and asset generation

source sync scene regeneration
  -> prepared decision payload
  -> the same pipeline contracts
  -> strict presentation projection validation
  -> replace scene records
  -> remove generation/media graph entries that reference replaced lines

project family import
  -> strict package validation
  -> the same galgame_line_from_model projection
  -> strict generation params validation
  -> canonical-key/ID remap
  -> DB triggers
```

## 生产数据与迁移证据

- 开发库使用现有 `a4n-postgres` 容器内 PostgreSQL CLI 只读审计，未写入开发数据。
- `galgame_line` 共 130 行；旧 `cg_prompt/event_cg_note` 字段 0 行，非 `galgame_line_presentation_v2` 0 行，CG 指令不完整 0 行。
- 现有 CG generation spec 4 条，缺失 `cg_importance` 0 条。
- 对白/独白 44 行，缺失或空 pose 0 行；立绘差分 generation spec 12 条，缺失或空 pose 0 条。
- 历史 v2 行仅缺少后来明确为 required-nullable 的键：`transition` 130 行、`after_beat_ref` 127 行、`cg/cg_note/cg_importance` 各 126 行、`branch` 100 行、`branch_id/returns_to_scene_index` 各 103 行。
- 隔离测试库 `a4n_test` 审计时 Galgame 相关表为空，不需要测试数据兼容转换。
- 因生产数据没有旧别名、非法 importance 或无 pose 变体，迁移只补齐缺失 nullable 键；遇到其他旧格式或非法行直接失败。

## 优化记录

- R-1261：以 Galgame 页面完整功能为起点完成 BFS，覆盖前端、Host、Runtime、Orchestrator、DB、source-sync、项目族传输、prompt、workflow 与 migration。
- R-1262：绘制并核对 `UI -> Host -> Runtime -> Orchestrator -> DB` 主拓扑，以及 pipeline、source-sync、资产和项目族四条活跃分支。
- R-1263：生产库只读审计确认 130 条行记录均为 v2、无 CG 旧别名、无不完整 CG 指令。
- R-1264：生产库只读审计确认 4 条 CG spec 均有显式 importance，不需要默认 `normal` 或兼容转换。
- R-1265：生产库只读审计确认 44 条说话行和 12 条立绘 spec 均有 pose，可删除 expression-only 旧变体。
- R-1266：删除 217 行纯透传 `GalgameRuntimeFacade` 及 Runtime state 模块，Runtime action 直接调用 Orchestrator application。
- R-1267：Runtime Galgame action 直接复用 `a4n-api-contract` DTO，删除本地 `GalgameProjectInput`、`DeleteGalgameLineInput` 和多组类型别名。
- R-1268：行、场、选项 mutation 的 Host 返回值统一为 API read model，不再向 transport 暴露 SeaORM model。
- R-1269：管线预览使用独立 `PreviewGalgamePipelineInput`，删除借用启动 DTO 造成的合同分叉。
- R-1270：Runtime 直接实现 prompt renderer port，删除仅包一层 state 的 `RuntimePromptRenderer` 临时适配。
- R-1271：Galgame 管线阶段、总步数和阶段 key 统一由 domain contract 所有，删除 Orchestrator 重复 stage 模块。
- R-1272：删除 DB 项目进度中的 Galgame 专用 pipeline 查询模块，复用共享 workflow 最新状态聚合。
- R-1273：删除 Runtime Galgame 媒体层重复验证 DB owner 的整文件测试，仅保留业务 owner 测试。
- R-1274：收紧 Orchestrator Galgame 公共可见性，只暴露 Runtime 实际消费的 application input、progress、preview 和 asset-generation port。
- R-1275：资产规划输出的 target kind/entity kind 改为领域枚举，删除跨层任意字符串透传。
- R-1276：Host、Runtime、Orchestrator、DB read model 的 Galgame line/scene/choice/media/asset 合同统一到 canonical API/domain 类型。
- R-1277：修复 pipeline JSON 对 `GalgameLinePresentationSchemaVersion` 的漏导出，删除调用方绕过领域合同的需求。
- R-1278：`GalgameLinePresentationDocumentV2` 的 nullable 字段改为 required-nullable，缺键不再由 Serde 隐式解释为 `None`。
- R-1279：新增 core presentation 到完整 document 的唯一 typed 转换，集中生成全部 nullable null 字段。
- R-1280：主线和支线持久化改为序列化完整 typed document，删除对部分 JSON 的增量拼接依赖。
- R-1281：choice 行演出数据改为完整 typed document，补齐 CG、transition、branch identity 和 return 字段的显式 null。
- R-1282：read-model、pipeline persistence 和 source-sync 共用严格 presentation 解析与投影一致性校验，删除只检查 JSON object 的弱门禁。
- R-1283：项目族 Galgame 行导入复用唯一 `galgame_line_from_model` 投影，删除第二套 presentation/行语义解析器。
- R-1284：资产规划入口先统一投影全部 DB 行，背景、立绘和事件 CG 规划共用同一 read model。
- R-1285：删除 `asset_plan/presentation.rs` 的手写 CG parser、旧字段遍历和 importance 默认逻辑。
- R-1286：彻底移除 `cg_prompt`、`event_cg_note` 旧字段别名；迁移检测到别名直接失败。
- R-1287：删除缺失/空 `cg_importance` 自动采用 `normal` 的 fallback，启用 CG 必须显式提供枚举值。
- R-1288：未启用 CG 时禁止残留 `cg_note/cg_importance`，删除“有 note 即视为启用”的旁路语义。
- R-1289：CG generation spec 的 importance 改为必填枚举，并删除无生产调用者的 optional importance validator。
- R-1290：事件 CG 资产规划直接消费 typed presentation，不再从任意 JSON 中二次解析 note/importance。
- R-1291：说话行语义统一要求 expression 与 pose 同时存在；staging、mutation、read-model、source-sync 和 DB trigger 使用同一规则。
- R-1292：立绘 variant 仅保留 `expression + pose` 身份，删除 expression-only 分支和 canonical key。
- R-1293：立绘 draft、generation params、asset-generation plan、Host port 和 DB target input 的 pose 全部改为必填字符串。
- R-1294：项目族导入校验和 ID remap 要求非空 pose，删除项目包对无 pose 旧 spec 的兼容 key 生成。
- R-1295：completed portrait 查询只按必填 pose 精确匹配，删除查询 `pose IS NULL` 的旧分叉。
- R-1296：合并媒体清理与 source-sync 清理的两套 `source_line_ids` 解析器，保留 generation owner 中的一份严格实现。
- R-1297：畸形、空、非正整数或重复 `source_line_ids` 不再静默当作“不匹配”，清理流程显式失败。
- R-1298：删除 Orchestrator source-sync 对 DB 纯解析 helper 的重复测试，由 a4n-db owner 唯一验证。
- R-1299：Galgame line PostgreSQL function 严格要求 v2 全字段、拒绝未知键、校验 nullable 类型和 CG 组合规则。
- R-1300：Galgame generation-spec PostgreSQL function 要求 CG importance 与立绘 pose 必填，并只接受带 pose 的 canonical key。
- R-1301：target reverse-integrity function 删除 CG 别名、importance 默认值和 pose 空值等价比较。
- R-1302：新增不可逆迁移 `galgame_presentation_contract_cleanup`，只给历史 v2 行补齐缺失 required-nullable 键。
- R-1303：迁移复用 line、generation-spec、reverse-integrity 三个 owner SQL 常量，不复制触发器合同正文。
- R-1304：迁移对所有 Galgame 行、CG spec 和立绘 spec 执行 no-op 更新，强制现有数据走严格触发器验证。
- R-1305：迁移注册为最后一项，并同步更新 `REQUIRED_SCHEMA_MIGRATION` 与迁移顺序测试。
- R-1306：删除项目族 Galgame presentation 专用校验模块，导入、read-model 与运行时只保留一个投影事实源。
- R-1307：read-model 测试修正 detail ID、完整 v2 fixture 和严格错误断言，避免旧夹具继续声明宽松合同。
- R-1308：pipeline persist 测试改为反序列化完整 document，并验证 choice branch/beat identity，不再断言字段缺失。
- R-1309：退役符号静态搜索确认旧 CG 字段解析、optional importance、optional pose variant、弱 presentation validator 和重复 source-line parser 生产引用为 0。
- R-1310：本专项坚持使用仓库现有 `target/`；累计超过 10 项后统一执行 Rust、数据库、API 与前端验证。
- R-1311：visual-plan 的事件 CG importance 在 Serde/Schemars 边界直接使用 `GalgameCgImportance`，不再把已校验值降级为任意字符串。
- R-1312：删除 Orchestrator `CG_IMPORTANCE`、domain `GALGAME_CG_IMPORTANCE` 两份字符串表和 visual-plan 手写 membership 校验，只保留领域枚举。
- R-1313：项目族 generation params 校验复用 `GalgameCgImportance::parse`，删除第三套 importance 取值事实源。
- R-1314：CG presentation 持久化直接写入 typed importance，删除跨层临时字符串转换。
- R-1315：资产生成媒体持久化返回唯一 Host `GalgameMediaBindingSummary`，不再声明实际不会返回的 DB result 类型。
- R-1316：read-model 的 line/scene/choice/media projection 统一从 `read_models` 模块内部出口提供；仅项目族确有调用的 line projection 保留 crate 可见性。
- R-1317：workflow 状态与取消查询显式使用 `Some(false)` 和 `Some(1)`，与共享查询合同一致，不引入默认值旁路。
- R-1318：删除 `participants_json` 对 `BTreeMap` 结果再次去重的不可能分支。
- R-1319：同一 staged line 的 speaker projection 只查找一次，删除 expression、pose、position 三次重复扫描。
- R-1320：删除只调用 `to_string()` 的 `stage_position` 包装函数，调用点直接使用领域枚举显示合同。
- R-1321：CG 查找参数由 `Option<&&T>` 收敛为 `Option<&T>`，删除无意义双重引用接口。
- R-1322：删除已无任何生产调用者的 228 行 `visual_params_rules` 整模块及其重复单测。
- R-1323：删除仅测试自身、未接入 LLM 活跃链路的 `parse_galgame_json_object` envelope parser 及 Markdown fence 测试。
- R-1324：删除 envelope 退役后唯一剩余的 `require_object_value` 死 helper。
- R-1325：workflow mode、business key 和 fingerprint 收紧为 owner 模块私有，只向 Galgame application 暴露 workflow kind 与 run input。
- R-1326：清理 Galgame 根模块无调用者的 API/domain/DB 重导出，只保留活跃调用链真实消费的合同。
- R-1327：清理 pipeline JSON 根模块无调用者的 draft/option/parser 重导出，测试专用 staged/draft 类型仅在测试构建可见。
- R-1328：迁移静态测试改为断言当前 `(target_kind, target_entity_kind)` 成对合同，删除对旧单字段 SQL 形态的过时依赖。
- R-1329：前端 CG 启用状态只认 typed `cg === true`，删除“存在 cg_note 即视为启用”的兼容推断。
- R-1330：删除前端编辑、启用、失焦和 JSON 更新中的五处 `normal` importance 自动补值，用户必须显式选择枚举。
- R-1331：CG 编辑只在 note 与 importance 同时完整时持久化；不完整组合显式提示并由纯函数拒绝。
- R-1332：立绘矩阵把 pose 改为必填读取，删除空 pose 和“默认姿势”展示分支。
- R-1333：选择支草稿不再猜测下一场或任意首场，新增选项必须显式选择目标场；删除 `defaultChoiceTargetSceneId`。
- R-1334：手工新增行改为显式输入“旁白”内容，删除“新的剧本行”占位持久化，并把操作标签明确为新增旁白。
- R-1335：未知 generation-spec 状态不再显示“状态未知”，直接暴露非法合同错误。
- R-1336：未知立绘 expression 不再降级为“其他表情”，直接暴露领域枚举违约。
- R-1337：抽出唯一 CG presentation 更新纯函数，并新增最小测试覆盖缺 importance 拒绝、显式启用和完整清除。

## 数据库迁移

`galgame_presentation_contract_cleanup` 为不可逆迁移：

- 仅接受 JSON object 且 schema 为 `galgame_line_presentation_v2` 的历史行。
- 发现 `cg_prompt/event_cg_note` 直接失败，不进行旧字段转换。
- 仅以 JSON null 补齐 `focus_character_profile_id/cg/cg_note/cg_importance/transition/branch/branch_id/returns_to_scene_index/after_beat_ref` 缺键，已有值不覆盖。
- 重新安装当前唯一的 line、generation-spec 和 reverse-integrity functions。
- 全量触发现有行、CG spec 与立绘 spec 校验；非法数据使整个迁移回滚。
- down 明确失败，不恢复旧别名、默认 importance、无 pose variant 或缺字段格式。

## 集中验证结果

- `cargo fmt --all`：通过。
- `cargo test -p a4n-domain galgame --lib -- --nocapture`：1/1 通过。
- 加载仓库 `.env` 后串行执行 `cargo test -p a4n-migration galgame --lib -- --nocapture --test-threads=1`：18/18 通过，其中数据库迁移测试验证 nullable 补齐与旧别名拒绝。
- `npm run check:web-api-parity`：通过，Host API、generation schema、TypeScript 输出和 fixtures 均为最新。
- `npm run test:run`：最终完整前端套件 45 文件、163 用例通过，包含新增 CG presentation 严格组合测试。
- `npx tsc --noEmit` 与 `npm run build`：通过，Galgame 生产 bundle 正常生成。
- `cargo check -p a4n-orchestrator --lib`：Galgame 代码无错误和警告；crate 仍被 character、import-reverse、shooting-script、blueprint、RAG、video 等既有错误阻断。
- `cargo check -p a4n-runtime --lib`：同样在依赖 Orchestrator 时被上述非 Galgame 错误阻断，未发现 Galgame/Runtime 新错误。
- `cargo test -p a4n-db galgame --lib -- --test-threads=1`：尚未进入 Galgame 用例，lib test 构建被 video `timeline_item_id` 与 blueprint-version 旧夹具字段等 16 个非 Galgame 错误阻断。
- `git diff --check`：通过。

## 当前事实标准

Galgame 只保留一套实现：Host 使用生成的 API/domain 合同；Runtime 只做 transport 与具体生成 Host 适配；Orchestrator 拥有权限、管线和业务投影；DB records 与 PostgreSQL trigger 共同保证持久化不变量。演出数据只接受完整 v2 document，前端和后端都要求 CG 显式提供 note/importance，立绘身份必须同时包含 expression/pose，选择支必须显式选择目标场，项目族和 source-sync 复用同一严格投影，不保留别名、默认值、弱解析或重复清理实现。
