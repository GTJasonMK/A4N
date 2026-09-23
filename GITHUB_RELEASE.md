# 源码检查与发布

## 当前发布能力

当前仓库只有 [Module Checks](.github/workflows/module-checks.yml) 和 [iOS IPA](.github/workflows/ios-ipa.yml) 两条 workflow。仓库没有自动创建 GitHub Release 的流水线；推送版本 tag 不会自动完成 Apple/Azure 签名、三平台发布或资产上传。

Module Checks 包含构建、测试、合同检查、供应链检查和平台打包 smoke。实际执行结果以对应 commit 的 CI 为准；测试包、未签名 IPA 与正式签名发布是不同交付物。

## 提交源码

先按[贡献指南](CONTRIBUTING.md)完成相关验证，再逐项检查工作区：

```bash
git status --short
git diff --check
git diff --stat
```

只暂存已审阅的文件，尤其注意工作区中其他任务的改动。迁移登记与迁移文件、共享合同与生成物、脚本与其检查必须一起交付。不要提交环境凭据、数据库、日志、缓存、生成媒体或签名材料。

暂存后检查 `git diff --cached --check` 和 `git diff --cached --stat`，再按仓库流程提交或创建 PR。文档是否被跟踪应使用 `git check-ignore -v PATH` 核实，不能仅凭文件存在判断。

## 本地打包

当前打包入口是 [scripts/deploy.sh](scripts/deploy.sh)，在开发工作树根目录运行；工作树要求见[贡献指南](CONTRIBUTING.md#并行工作树与集成基线)，首次环境准备见[源码运行指南](docs/public/getting-started.zh.md)：

```bash
./scripts/deploy.sh --help
./scripts/deploy.sh --dry-run
./scripts/deploy.sh
./scripts/deploy.sh --no-build
```

脚本从 service/tool catalog 派生打包清单，产物为 `deploy/a4n-tauri-<version>-<platform>.tar.gz`。`--services export,video` 选择要分发的 sidecar；它不代表功能包权限或安装生命周期已经完整验收。`--target` 指定打包平台标识，跨平台编译仍须使用相应工具链和平台验证。

iOS 构建与侧载限制见 [ios/README.md](ios/README.md)。云端部署由 [deploy-cloud.sh](scripts/deploy/deploy-cloud.sh)单独处理，不等同于 GitHub Release 发布。

## 云端更新

配置工作树的 `.deploy.env` 后运行 `./scripts/deploy/deploy-cloud.sh`。自动模式比较上次部署保存的源文件 SHA-256，覆盖已提交、未提交和删除的内容；旧部署记录缺少源文件快照时，首次更新会执行全量构建并建立基线。`--full` 可显式要求全量更新。

云端 Rust 产物位于 `$CARGO_TARGET_DIR/deploy-cloud/target`，容器使用当前用户的 UID/GID，并以只读方式挂载源码。容器 Cargo 元数据位于相邻的 `cargo-home`；离线模式只读复用本机 `${CARGO_HOME:-$HOME/.cargo}` 的 registry/git 缓存。`CARGO_TARGET_DIR` 保持调用者设置，默认是当前工作树的 `target`。

更新脚本后需先完成一次构建，再使用 `--skip-build` 复用新目录中的产物。云服务器仍应配置固定监听地址（默认 `127.0.0.1:8080`），供反向代理与健康检查访问。

## 正式对外分发

正式发布前，应从同一已验证 commit 构建，核对版本、完整性 manifest、SHA-256、所选服务和平台运行结果；签名与公证必须有该次产物的真实验证证据。平台工具与检查见 `scripts/checks/`，不要用其他版本的历史测试记录代替。

需要自动 GitHub Release 时，先实现并审查发布 workflow、最小权限、签名凭据注入、失败时保留 draft 和下载后复验，再为其编写部署说明。当前没有固定的“12 个发布资产”或自动公开 draft 的保证，也不应按旧指南向不存在的流水线配置 secrets。
