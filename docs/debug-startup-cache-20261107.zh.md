# debug 启动重复构建修复（2026-11-07）

已完成。基线为 `7bf6f53437ada2263e35568062a9b598bb0d7b48`，按用户授权直接在 debug 修改，不新建分支。改动限于开发构建入口、回归测试及说明文档，没有修改 Rust 业务源码、前端页面、依赖或 API 合同。

## 根因与修复

### 迁移阶段隐藏的单包构建

原 Web 启动流程先统一构建本地运行组件，再调用 `a4n local migrate`。该 CLI 命令在源码模式通过 `resolve_or_build_binary` 再执行 `cargo build -q -p a4n-migration`，改变公共依赖的构建指纹。随后启动 Server 时又回到另一套依赖图。

完整启动复现中，两次源码未变的启动均在 Server 阶段重新编译 44 个 crate，分别耗时 51.09 秒和 48.02 秒；CLI 内部的 `-q` 构建不出现在外层启动日志中。此前仅检查统一构建入口的 no-op，并不能覆盖这条实际调用链。

现在统一入口支持直接执行 `a4n-migration`，启动脚本使用已通过 Cargo 新鲜度检查的迁移器，迁移失败仍停止启动。迁移与 Server 使用相同的 workspace manifest、稳定包顺序及 profile；`--release` 同时传给迁移与 Server。

### Runtime 热更新切换依赖图

旧 watcher 按变化目录单独选择 package。实测单独构建 Control 需要生成另一组产物；回到完整启动构建又重编译 44 个 crate，耗时 50.98 秒。Cargo 指纹日志明确记录公共依赖状态变化，源码内容未变。

现在 watcher 调用已有的统一开发构建入口，由 Cargo 判断实际过期的目标，删除 watcher 内第二套包选择逻辑。原事件合并、失败后保留运行产物及成功后发布 reload marker 的行为保留，不再用自建 Rust 缓存跳过 Cargo。

### 前端无条件打包

Web、桌面 `--no-hot` 和 `scripts/build.sh` 现在共用 `scripts/dev/build-frontend.mjs`。缓存输入包含源码、公共资源、配置、依赖锁文件、Node/平台及相关构建环境，按内容判断，不按文件时间判断；同时校验 dist 文件的内容与完整性。

成功凭据位于 `frontend/node_modules/.cache/a4n/frontend-build.json`，只保存摘要。构建前清除旧成功凭据，失败、缺少 index.html 或构建期间输入变化时不记录成功。缓存命中不改写产物；文档和 E2E 文件不触发打包。直接执行 `npm --prefix frontend run build` 仍可强制打包。

## 关键文件

- [启动入口](../scripts/run.sh)：直接运行迁移器，共享前端缓存入口。
- [Cargo 运行入口](../scripts/dev/run-cargo-targets.sh)：统一 manifest、包顺序和运行目标，保留调用者的目标目录、features 与 profile。
- [Runtime watcher](../scripts/dev/watch-runtime-workers.sh)：复用启动构建图。
- [前端构建入口](../scripts/dev/build-frontend.mjs)及[构建脚本](../scripts/build.sh)：内容指纹与产物校验。
- [前端缓存回归](../scripts/tests/test_frontend_build_cache.py)、[构建参数回归](../scripts/tests/test_run_cargo_targets.py)、[watcher 回归](../scripts/tests/test_runtime_worker_watcher.py)。

## 实际验证

证据位于 `data/startup-cache-20261107/`，未纳入 Git。

| 验证 | 结果 | 证据 |
| --- | --- | --- |
| Python 全组 | 176 项通过，每组以 60 秒超时运行 | `python-unit-final-verified.log` |
| 缓存与启动回归 | 前端缓存、事件合并、失败不 reload、profile/feature 转发及预检失败保护均覆盖 | 上述单测与 `startup-unit-final.log` |
| 前端实际构建 | 首次 22.889 秒；第二次 0.248 秒，未调用 npm build，276 个产物的字节与 mtime 全部保持一致 | `frontend-cache-real.json` |
| 真实文件监听 | 受控源码变更及恢复各只编译 Control；没有编译其他 crate，临时源码改动已恢复 | `real-watcher.log`、`real-watcher-summary.json` |
| 完整启动两轮 | 迁移和 Server 的四次 Cargo 检查为 0.37 / 0.38 / 0.39 / 0.37 秒；Rust 重编译为 0，前端打包为 0 | `startup-1.log`、`startup-2.log` |
| 服务就绪 | 两轮分别 9.784 秒、6.774 秒，包含真实迁移检查和组件启动 | `startup-smoke.json` |
| CLI / 浏览器 | 10 项 CLI 检查及两次真实浏览器登录、项目列表检查通过；无页面异常 | `startup-smoke.json`、`browser-1.log`、`browser-2.log` |
| 隔离环境与清理 | 临时 PostgreSQL、独立 Qdrant 数据与端口、本地虚拟 Provider；退出后回收测试容器和进程 | `smoke-db-final-isolated/database.json` |
| 桌面调用合同 | 实际捕获 Tauri 的 runner 参数，回归验证 no-default-features、migration feature、目标目录和应用参数分隔 | `tauri-runner-args.json`、`tauri-build-contract-unit.log` |
| 工具链检查 | 测试目录、Bash 语法、Node 语法、cargo fmt 与 diff 检查通过 | `catalog-final-verified.log`、`format-final.log` |

保留了定位过程的失败：最初 Qdrant 测试配置扩展名不受支持，健康检查失败；第一次完整验收复现迁移后的 44 个 crate 重编译；后续一次使用旧 CLI 配置被地址冲突保护拒绝。修正测试配置并为每轮分配独立 CLI 状态后，最终完整启动验收通过。没有删除失败日志或放宽缓存、登录冲突及页面断言。

## 边界

- 首次建立前端缓存、切换 Node / Rust 版本、features、profile 或修改真实输入时仍会正常构建。缓存命中不等于跳过迁移、权限或运行依赖检查。
- 日常启动保留调用者的编译配置，没有把用户的 debug 符号、增量编译或目标目录强行改成验收设置。
- 实际浏览器验收使用 Web 模式；桌面验证覆盖 Tauri 调用合同，本轮未重新运行原生窗口和完整 release 打包。
- 仅验证启动与缓存相关行为，没有重跑全部业务生成用例，也未调用外部付费模型。Agent RAG 未开展。
