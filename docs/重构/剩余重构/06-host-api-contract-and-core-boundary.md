# 工作包 06：Host API 合同与 a4n-core 边界

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 建立唯一 Host API Rust 合同源，并用 public-type inventory 阻止 `a4n-core` 继续膨胀 |
| V2 来源 | 3.12、3.24、阶段 C、阶段 L 的 core/API 部分 |
| 当前状态 | 本地关闭条件已满足：`a4n-api-contract` 已建立并完成 21 个领域批次迁移；API inventory、生成检查、前端重复类型、parity 和 core public-type 门禁均已通过。仍需协调者在文档进入 Git/CI 证据齐备后回写 V2 3.12/3.24 |
| 可并行性 | DTO/core inventory 可立即并行；实际合同迁移会触及 Server/Tauri/frontend，需协调者分批合并 |

## 架构约束

- 创建 `a4n-api-contract`，不创建 `a4n-contracts`。
- 跨进程 sidecar 合同继续唯一归 `a4n-service-protocol` 或 capability contract。
- `a4n-api-contract` 不依赖 SeaORM、DB、provider、runtime、orchestrator、Tauri 或 Axum。
- Runtime 内部状态、Host 私有配置和仅一个入口使用的适配类型不迁入公共合同。
- TypeScript 必须从唯一 Rust 合同生成或受同一 schema 门禁约束，不能维护第二份手写字段定义。

## 执行内容

### 1. DTO inventory

1. 扫描 Runtime、Server、Tauri、frontend 的公开请求/响应和转换点。
2. 为每个类型标记：`public_host_contract`、`runtime_internal`、`sidecar_contract`、`domain_value`、`duplicate`。
3. 记录 Web/Tauri endpoint、frontend 消费者、serde wire、默认值和兼容要求。

### 2. 建立最小合同 crate

1. 只加入第一批真实公共请求/响应和必要的无依赖共享值类型。
2. Server、Tauri、Runtime Host adapter 使用同一类型；删除重复 DTO 和字段转换。
3. 增加 TypeScript 生成/校验入口，保持 Web API parity 为 0。
4. 按领域小批迁移，禁止一次性搬迁 352 个类型。

### 3. Core public-type inventory

1. 分类 core 公开类型：SeaORM entity、基础配置、基础事件、无依赖共享值、Host DTO、sidecar DTO、领域规则。
2. Host DTO 迁入 `a4n-api-contract`；sidecar DTO 迁入既有 protocol/capability；领域规则留在 `a4n-domain`/领域 crate。
3. 扩展 `check-core-boundary.sh` 和 allowlist，使新增公开类型必须声明类别与理由。
4. 保持 core 不依赖 runtime/orchestrator/provider/service。

## 文件所有权

```text
crates/a4n-api-contract/**                     # 新 crate，最小依赖
crates/a4n-runtime/src/host_api/**             # Host 适配
crates/a4n-server/src/**                       # Web 映射
crates/a4n-tauri/src/**                        # Tauri command 映射
frontend/src/api/**                            # 生成类型消费者
crates/a4n-core/src/**                         # 只做分类和必要迁出
scripts/checks/check-core-boundary.sh
scripts/checks/a4n-core-public-*.txt
```

workspace `Cargo.toml`、`Cargo.lock`、frontend 生成入口和 CI workflow 属共享文件，由协调者合并。

## 不在范围

- 不把业务规则或 application service 塞进合同 crate。
- 不改变 sidecar wire 合同 owner。
- 不顺手重命名全部 API 或制造 v2 endpoint；只迁 owner，除非现有字段确有冲突。
- 不因 TypeScript 生成方便而把 runtime 内部状态暴露给前端。

## 关闭条件

- 所有 frontend 可见请求/响应有唯一 Rust owner。
- Server/Tauri 使用同一公共合同或明确的 Host 私有适配类型。
- TypeScript 生成/校验和 API parity 进入 CI，缺失为 0。
- core public-type inventory 能阻止新增非允许类别；Host/sidecar/领域类型各归正确 crate。
- 删除重复 DTO、第二套 schema 和无消费者转换；兼容行为有合同测试。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
cargo metadata --locked --no-deps --format-version 1
timeout 60 cargo check -p a4n-api-contract
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo check -p a4n-server --no-default-features
timeout 60 cargo check -p a4n-tauri --no-default-features
cd frontend && npm run check:web-api-parity
cd frontend && npm run build
./scripts/checks/check-core-boundary.sh
```

## 交接要求

附 DTO 分类清单、迁移批次、兼容变化、生成产物和 core 类型分类差异。只有 inventory 和第一批迁移完成时，不得把 3.12/3.24 标为完成。

## 本次执行记录（2026-07-25）

### 结果

- 新增唯一合同 crate：`crates/a4n-api-contract`，依赖仅为 `serde`、`serde_json`、`schemars`；边界脚本禁止 DB、SeaORM、Runtime、Orchestrator、provider、Tauri、Axum 和 sidecar 依赖。
- 完成 21 个迁移批次，详见 [`06-host-api-migration-batches.tsv`](06-host-api-migration-batches.tsv)：Theme、Auth、Config、Logging/Storage、Import、Reverse Import、Request Preview、Embedding、Admin、Image、Galgame、RAG、Project Transfer、Import Review、Canvas、Optimize、Project、Profile、Shooting Script、Source Sync、Video。
- Canvas、Shooting Script、Source Sync、Video 中依赖 DB/Core/生成媒体的部分均只在 Runtime Host 边界显式投影；合同 crate 不持有这些领域类型，也没有新增兼容 facade。
- TypeScript 生成产物统一位于 `frontend/src/api/generated/{theme,auth,config,host}.ts`；前端 API 类型改为生成类型的直接别名或最小字面量收窄。

### 当前 inventory

由 `scripts/checks/generate-host-api-dto-inventory.mjs` 生成并通过 `--check`：

```text
public_host_contract 75
runtime_internal     21
domain_value         14
frontend-visible or duplicated public_host_contract: 0
```

`a4n-core` 公共声明 inventory 为 303 项，类别清单和模块/依赖边界门禁均通过；本包没有把 Core 的领域类型搬进合同 crate。

本批将 09 会话新增的 `models/generation/generation_start_request.rs` 与 `models/generation/generation_task_checkpoint.rs` 各自的 `Relation`/`Model` 两个公开 SeaORM 声明补入 `scripts/checks/a4n-core-public-types.tsv`。这四项继续归类为既有 `models/` → `seaorm_entity`，没有迁移业务 owner，也没有扩大合同 crate。

### 已执行验证

```text
rustfmt（本包修改文件）：通过
cargo check -p a4n-api-contract -p a4n-runtime --no-default-features：通过
cargo check -p a4n-server --no-default-features：通过
cargo check -p a4n-tauri --no-default-features：通过
frontend npm run build：通过（2178 modules transformed）
frontend npm run check:web-api-parity：通过（缺失映射 0）
cargo metadata --locked --no-deps --format-version 1：通过
git diff --check：通过
node scripts/checks/generate-host-api-dto-inventory.mjs --check：通过
scripts/checks/check-api-contract-boundary.sh：通过
scripts/checks/check-core-boundary.sh：通过（303 declarations / 22 modules）
```

### 尚待协调者聚合

- 本次 `cargo fmt --all -- --check` 仍会命中其他并行会话尚未格式化的无关文件；本包新增/修改 Rust 文件已逐文件 `rustfmt --check` 通过，因此未执行全库格式化以免覆盖并行改动。
- 完整 workspace/CI 门禁和共享工作区合并仍由协调者负责；不因此单独把 V2 3.12、3.24 或阶段 C/L 写成完成。

### 历史并行复核交接（2026-07-25）

- `scripts/checks/check-api-contract-boundary.sh`：通过。
- `node scripts/checks/generate-host-api-dto-inventory.mjs --check`：通过。

历史记录：当时 `check-core-public-types.sh` / `check-core-boundary.sh` 因 09 会话新增的 4 个 generation model 声明未入 snapshot 而失败；批次 265 已按交接动作补齐快照并取得通过结果。
### 批次 265 · generation public-type inventory 交接收口（2026-07-26）

- 保留 `generation_start_request` 与 `generation_task_checkpoint` 在 `a4n-core` 的 SeaORM entity owner；只把四个实际 public declaration 加入快照，不复制类型、不移动 DB owner。
- `models/` 已有 `seaorm_entity` 分类覆盖这四项，无需新增类别或放宽 core 依赖边界。
- 这解决了 09→06 的唯一静态门禁阻塞；没有把该局部收口冒充 V2 3.12/3.24 或整体重构完成。

### 批次 265 验证

```text
scripts/checks/check-core-public-types.sh：通过（303 declarations）
scripts/checks/check-core-boundary.sh：通过（303 declarations / 22 modules）
```
