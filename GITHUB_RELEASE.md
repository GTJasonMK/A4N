# 源码检查与发布

## 桌面端发布

[Desktop Release workflow](.github/workflows/release.yml) 为以下平台构建系统安装包：

- Windows x64：MSI
- macOS Apple Silicon：DMG
- Linux x64：deb、rpm

工作流同时构建 Tauri updater 所需的签名更新资产和 `latest.json`。它会创建或更新 GitHub draft Release，并在校验后保留为草稿；维护者需在 GitHub Releases 页面审核并手动发布。桌面应用从 `crates/host/a4n-tauri/tauri.conf.json` 配置的地址读取已发布的更新清单。

Release tag 必须与 `crates/host/a4n-tauri/Cargo.toml` 中 `a4n-tauri` 的版本一致，例如版本 `0.1.1` 对应 tag `v0.1.1`。运行工作流前，在仓库 Actions Secrets 中配置：

- `TAURI_SIGNING_PRIVATE_KEY`：Tauri updater 私钥内容；不要提交到仓库。
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥设置了密码时填写。

updater 公钥保存在 Tauri 配置中。正式构建缺少私钥会失败。工作流会用公钥验证 updater 签名，上传资产到 draft Release，再从 Release 下载资产并复验签名与 SHA-256。

发布流程：

1. 更新 `crates/host/a4n-tauri/Cargo.toml` 版本并提交发布改动。
2. 推送匹配的 `vX.Y.Z` tag，或在 Desktop Release 中手动选择已存在的 tag。
3. 审核 draft Release 中的安装包、更新清单和校验结果，然后在 GitHub 上手动发布。

首期工作流未配置 Windows Authenticode，也未配置 Apple Developer ID 签名和公证。macOS 用户可能需要在 Gatekeeper 中批准应用，Windows 可能显示 SmartScreen 提示。Tauri updater 签名不等同于这些操作系统发行者信任检查。

## 更新中的数据迁移

工作流通过 `a4n-migration release-data-migrations` 生成 `data_migrations` 清单，迁移 crate 是该元数据的唯一来源。新增数据迁移时，需要同时登记名称、实现预检、完成状态检查和可重试的执行逻辑；发布清单与应用更新流程读取同一登记。

有待执行的数据迁移时，应用会先检查活动生成任务并运行迁移预检，然后创建 PostgreSQL custom 格式备份到应用数据目录的 `update-backups/`。备份会通过 `pg_restore --list` 校验，并记录 SHA-256。此类更新要求 `PATH` 中有兼容的 `pg_dump` 和 `pg_restore`。系统保留最近两份已验证备份。更新失败时保留备份和恢复状态，不会自动覆盖数据库进行整库恢复。

新版本启动时会先执行恢复状态中记录的数据迁移。迁移失败会保留恢复状态和备份，以便诊断或下次启动重试。纯 schema 更新不创建数据库备份。

## 本地打包与云端部署

`scripts/dev/build-tauri-bundle.sh` 用于开发和 CI 的原生 Tauri 打包；正式桌面安装包由 Desktop Release 工作流构建。`scripts/deploy.sh` 仍生成内部使用的便携包，不属于 GitHub 正式桌面发布渠道。

云端部署仍由 [deploy-cloud.sh](scripts/deploy/deploy-cloud.sh) 和服务端包单独处理。不要用桌面安装包替换服务端部署归档。

## 源码提交

发布前按[贡献指南](CONTRIBUTING.md)完成相关验证，并检查工作区和 diff：

```bash
git status --short
git diff --check
git diff --stat
```

不要提交 `.env`、数据库、日志、生成媒体、构建缓存或签名材料。确认文档是否被跟踪时使用 `git check-ignore -v PATH`，不要只根据文件存在与否判断。
