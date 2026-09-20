# 贡献指南

本页统一维护 A4N 的开发、目录与协作规范。安装运行见 [README](README.md)，模块边界以各 crate 的 manifest、源码和 Host 合同为准。

## 核心原则

- **以创作流程和用户内容为中心。** 正文、档案、媒体及其关联是业务数据；缓存和临时产物另行管理，清理与迁移必须保护用户内容。
- **事实和规则有明确归属。** 同一业务规则、权限规则、状态转换和补偿逻辑由一个明确的模块负责，其他层调用它；不同生命周期的状态不能为了“统一”强行合并。
- **用最小完整改动解决根因。** 先读真实调用链，复用现有实现、标准库和已有依赖；有实际需求再增加抽象、配置或依赖，不保留无人使用的兼容分支。
- **失败显性，交付有证据。** 边界输入显式校验，SQL 参数化；应用校验与必要的数据库约束保持一致。不得用吞错、假成功或隐藏 fallback 掩盖问题，也不把设计目标和历史测试成绩当作当前实现。

## 目录管理

| 位置 | 放什么、如何维护 |
| --- | --- |
| `crates/` | 按稳定职责划分 Rust crate；实现在各自 `src/`，单测靠近模块，集成测试放 crate 的 `tests/`。新代码先归入已有职责，不新建万能 `core` 或 `utils` 容器。 |
| `frontend/src/` | `pages/` 组合页面，`components/` 放共享组件，`api/` 集中有类型的后端访问，`assets/` 放静态资源；测试使用 `*.test.ts` / `*.test.tsx`。生成文件修改源合同或生成器后再生成。 |
| `prompts/` | 运行时提示词资产；模板与注册表同步，不按普通说明文档去重。 |
| `scripts/` | 按 `dev/`、`checks/`、`tests/`、`contracts/`、`deploy/` 归类；复用根入口和 `lib/` helper，避免复制启动、环境解析和清理逻辑。 |
| `work/docs/` | 本机文档、草稿、架构图和历史记录；该目录被忽略，不作为仓库输入。 |
| `data/` | 运行数据、用户媒体和日志等受保护内容，不提交到 Git，不当作构建缓存清理。 |
| `target/` | 可再生构建产物与短期验证输出；临时目录有明确清理责任，长期证据移出可清理缓存范围。 |

新增文件放在所属模块附近；根目录保留公共配置和入口，不堆放一次性报告。文件名表达职责，避免用 `new`、`final`、`v2` 文件复制代替正式迁移。本机文档统一放在被忽略的 `work/docs/`，不作为代码或发布输入。

## 架构边界与拆分

- 纯领域值、身份、校验和算法使用 `a4n-domain` 或已有专用 crate；配置、文件、事件等能力回到各自 owner，不往公共 crate 倾倒业务实现。
- 业务用例、工作流和副作用顺序由 `a4n-orchestrator` 编排；SQL、Repository 和数据库一致性边界属于 `a4n-db`，SeaORM 实体属于 `a4n-db-models`。
- Provider、文件、进程与 Host 适配位于 `a4n-runtime`；Server、Tauri、CLI 和前端复用已有业务入口，不分别实现一套业务规则。
- Host API descriptor/schema 属于 `a4n-api-contract`，通用 sidecar 协议属于 `a4n-service-protocol`，能力专用合同属于相应 `a4n-*-contract`；新增能力合同不能全部塞回通用协议。

文件拆分用于组织阅读，crate 拆分改变依赖与编译边界，进程拆分提供运行隔离。拆分依据是职责、真实消费者及测得的编译或运行收益；共享事务和状态不变量保持内聚。完整模块化仍在推进，不能承诺所有功能已可独立替换，或修改一个文件只重编该文件。

## 开发环境

需要 Rust、Node.js 22、npm、PostgreSQL 16+，Linux 桌面构建还需要 WebKitGTK、GTK 和 Ayatana AppIndicator 开发库。

```bash
./scripts/dev/dev-db.sh up
./scripts/dev/setup-onnxruntime.sh
npm --prefix frontend ci
./scripts/run.sh           # Tauri 桌面开发
./scripts/run.sh --server  # Web Server 开发
```

从仓库根目录运行这些命令，启动方式按需选择；平台依赖、ONNX/Qdrant 和环境变量以 README 为准。密钥使用被忽略的配置或环境变量，不写入源码、示例和日志。

## 开发流程与代码风格

1. 查看 `git status --short` 和 `work/docs/work.md`，确认已有改动与共享文件负责人，保留无关改动。
2. 明确用户可见行为、验收条件和负责模块；读取实现、调用方及测试，先确认旧测试仍符合要求。
3. 在正确 owner 修复根因，保持函数短、嵌套浅、错误清楚；注释解释意图和取舍，不复述代码。
4. 修改接口时同步源合同、descriptor、Server 分派及受影响的 Tauri/CLI/前端消费者。Host 类型用 `npm --prefix frontend run generate:api-contract` 生成；generation 合同用 `npm --prefix frontend run generate:generation-contract`，不手改生成结果。
5. 按影响范围验证，同步有效指南；依赖变化同步 manifest 和锁文件，边界变化同步检查规则与测试。

### 并行工作树与集成基线

- `debug` 是实际运行和调试基线，不在该工作树直接编辑、rebase、reset 或切换分支。
- 每个修改会话从最新 `debug` 创建独立的 `agent/<topic>` 分支和工作树；提交前只交付自己负责且已验证的完整切片。
- 集成在独立的 `integration` 工作树进行：先把 agent 分支合并到 `integration`，再运行受影响的编译、测试、格式和边界检查；验证通过后提交合并结果。
- `debug` 只接受验证通过的 integration 结果，并使用 `git merge --ff-only integration` 快进更新。`master` 和远端发布属于单独的明确交接窗口，不在普通调试合并中隐式更新，也不强推。
- `work/docs/work.md` 是被忽略的本地协作记录；公共规则以本文件、README、代码合同和测试入口为准。

Rust 使用 rustfmt、4 空格、`snake_case` 函数/模块和 `PascalCase` 类型，crate 名沿用 `a4n-<职责>`。TypeScript/TSX 使用 2 空格、双引号、分号、`camelCase` 函数和 `PascalCase` 组件/类型；沿用所在模块风格，不附带全仓格式重排。

## 验证要求

验证按改动风险选择；下表是对应范围的要求，不是每次全部执行的清单。

| 改动范围 | 验证要求 |
| --- | --- |
| 所有改动 | `git diff --check`；纯文档还要核对链接、路径和命令来源，无需重建应用。 |
| Rust 实现 | rustfmt 检查、受影响 crate 的编译检查与目标测试；使用 `#[test]` / `#[tokio::test]` 覆盖改变的不变量和失败路径，后端测试默认 60 秒超时。 |
| 前端 | 相关 Vitest、`check:web-api-parity` 和 `build`；交互或视觉变化还要实际打开桌面/窄屏页面，验证操作、键盘和布局，不能把服务启动当验收。 |
| 数据库 | 显式配置独立的 `A4N_TEST_DATABASE_URL`，按测试入口和测试代码串行运行会修改 schema 的测试；禁止对开发数据库重建或清理 schema。 |
| 用户命令、工作流、API | 执行对应的 `scripts/tests/run_*e2e.py`；浏览器链路使用 `scripts/tests/run_browser_full_project_e2e.py`。 |
| 数据库边界、sidecar、发布与供应链 | 执行对应 `scripts/checks/` 门禁，并记录实际覆盖范围。 |

常用验证示例；将 `a4n-queue` 换为受影响的 crate，按上表选择命令：

```bash
cargo fmt --all -- --check
./scripts/cargo.sh check --locked -p a4n-queue
timeout 60s ./scripts/cargo.sh test --locked -p a4n-queue -- --test-threads=1
npm --prefix frontend run test:run
npm --prefix frontend run check:web-api-parity
npm --prefix frontend run build
git diff --check
```

整体构建使用 `./scripts/build.sh`，后端编译检查使用 `./scripts/build.sh --backend --check`；复杂 Cargo/test 参数走 `scripts/cargo.sh`，`scripts/test.sh` 只支持 `--crate` 和 `--nocapture`。不能把超时、跳过、旧二进制或仅编译成功写成当前行为验证通过；无法验证时说明原因和剩余范围。

## 构建缓存管理

日常使用 `scripts/build.sh`、`scripts/run.sh`、`scripts/test.sh` 和 `scripts/cargo.sh`，复用现有缓存维护。保持常规编译选项与主 target 稳定；长期额外 target 登记到 `rust-artifacts.json`，临时测量 target 在相关进程退出后回收，成功或失败都保留必要报告。

预算与保护期以 [rust-artifacts.json](rust-artifacts.json) 为准，使用 `cargo artifacts inspect` 和 `df` 判断实际占用。总量预算是软预算，构建中、近期写入及受保护产物可能暂不回收；Btrfs 快照还可能保留旧 extent，文件统计减少不等于物理空间释放。

## 协作、提交与 Pull Request

多人协作时先明确文件归属，共享 manifest、lockfile、migration、合同及启动装配串行集成。完成后在工作记录写明范围、实际验证与未完成项；不回滚、覆盖或整理他人的无关改动。

历史中有 `optimize`、`refactor:`、`fix:` 等提交，新提交优先使用有具体描述的 Conventional Commits，例如 `fix: preserve workflow recovery state`、`docs: clarify cache cleanup rules`。

按 [PR 模板](.github/PULL_REQUEST_TEMPLATE.md)说明问题、行为变化、范围与 owner、相关 issue、数据库/API/协议变更、实际验证和剩余风险；可见 UI 变化附截图。不要提交 `.env`、数据库、日志、PID、构建产物、生成媒体、签名证书或本机运行数据。变更命令、目录职责或合同后同步唯一维护入口，删除文档时同步检查 Markdown、HTML、SVG 和 Skill 引用。

除非明确书面声明为“Not a Contribution”，提交到本仓库的贡献按项目 [Apache License 2.0](LICENSE) 提供。
