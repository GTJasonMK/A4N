# GitHub 发布说明

本文区分“推送源码仓库”和“创建签名 Release”。首次推送源码不需要 Apple/Azure 签名身份；只有推送 `v*` tag 才会触发正式签名发布。

## 1. 推送源码前

项目采用 [Apache License 2.0](LICENSE)。推送前检查工作树，确认全部删除、修改和新增均属于本次重构，不包含本机文件：

```bash
git status --short
git diff --check
git check-ignore .env data target deploy
! git check-ignore docs/TEST_DATABASE.md scripts/deploy/deploy-cloud.sh
git diff --stat
```

`.env`、`data/`、`target/`、`deploy/`、数据库、PID、日志、证书和签名文件必须保持 ignored。确认后由仓库所有者暂存并复核：

```bash
git add -A
git status --short
git diff --cached --check
git diff --cached --stat
git diff --cached --name-only
```

不要只提交 `migrator.rs`：`crates/a4n-migration/src/migrations/` 下新增且被注册的 migration 文件必须进入同一提交，否则已应用数据库会再次出现“migration file is missing”。重构计划、进度记录、工作包、workflow 和 checker 也必须一起进入提交。

确认暂存区后再提交并推送：

```bash
git commit -m "refactor: complete modular architecture migration"
git push origin master
```

## 2. GitHub 仓库设置

在仓库设置中完成：

1. **Actions > General**：允许仓库 workflow 运行；允许正式 release job 请求 `contents: write`。
2. **Environments**：创建名为 `release` 的 environment，建议启用 required reviewers，限制只有 `v*` tag 可以部署。
3. **Code security**：启用 Dependabot alerts、dependency graph、secret scanning 和 private vulnerability reporting；公开仓库可直接使用 GitHub 原生 secret scanning。
4. **Branches/Rulesets**：保护默认分支，要求 PR、分支最新并通过 `Module Checks` 后才能合并。

源码首次推送后先等待 `.github/workflows/module-checks.yml` 完成。不要在该 workflow 还没有当前提交的成功记录时创建正式 tag。

## 3. `release` Environment 配置

macOS secrets：

| 名称 | 内容 |
|---|---|
| `APPLE_CERTIFICATE` | Developer ID Application `.p12` 的 base64 内容 |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` 密码 |
| `APPLE_ID` | Apple ID |
| `APPLE_PASSWORD` | Apple app-specific password |
| `APPLE_TEAM_ID` | Apple Developer Team ID |

Windows secrets：

| 名称 | 内容 |
|---|---|
| `AZURE_CLIENT_ID` | Azure service principal client ID |
| `AZURE_CLIENT_SECRET` | Azure service principal secret |
| `AZURE_TENANT_ID` | Azure tenant ID |

Windows variables：

| 名称 | 内容 |
|---|---|
| `AZURE_ARTIFACT_SIGNING_ENDPOINT` | 官方 `https://*.codesigning.azure.net` endpoint |
| `AZURE_ARTIFACT_SIGNING_ACCOUNT` | Artifact Signing account name |
| `AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE` | certificate profile name |

这些值只配置在 GitHub `release` environment，不写入 `.env`、workflow、Issue、PR 或日志。Apple/Azure 任一必需值缺失时，正式 workflow 会按设计失败，不会生成未签名的替代 Release。

## 4. 创建正式 Release

正式 tag 必须与 `crates/a4n-tauri/tauri.conf.json` 的版本完全一致。先确认默认分支 CI 已通过，再创建 annotated tag：

```bash
version="$(ruby -rjson -e 'puts JSON.parse(File.read("crates/a4n-tauri/tauri.conf.json")).fetch("version")')"
git tag -a "v$version" -m "A4N v$version"
git push origin "v$version"
```

`.github/workflows/release.yml` 会按以下唯一顺序执行：

```text
supply-chain -> Linux x86_64 -> macOS arm64 -> Windows x86_64
```

全部成功后，GitHub Release 应包含 CycloneDX 归档、Linux archive、macOS ZIP/DMG/manifest/CMS、Windows archive 及相应 SHA-256。CycloneDX 归档包含 Host、release tool catalog 声明的工具与 service catalog 声明的 sidecar 对应的跨目标 SBOM；workflow 会重新下载这些资产复验，不要手工上传同名资产绕过验证。

macOS job 只创建私有 draft 并暂存已验证的 Linux/macOS/SBOM 资产；Windows 签名、manifest、smoke 和下载复验全部成功后，最终 job 会核对恰好 12 个资产，再把 draft 转为公开 Release。任一平台失败时 draft 保持未发布，不会暴露缺少 Windows 资产的不完整版本。

正式 workflow 的四个 checkout 都禁用凭证持久化；构建、npm、Cargo、migration、smoke 和签名步骤无法从 Git 配置继承发布 token。`GH_TOKEN` 只显式注入 draft 暂存、资产下载校验和最终公开步骤。

## 5. 发布失败时

- 源码 CI 失败：修复默认分支并重新运行，不创建 tag。
- environment 审批等待：由配置的 reviewer 审批，不取消签名步骤。
- secret/variable 缺失：补齐 GitHub environment 配置后重新运行失败 job。
- tag/version 不一致：删除未发布的错误 tag，修正版本或 tag 后重新创建；不要放宽 workflow 校验。
- Apple/Azure 身份不可用：只发布源码，不创建正式二进制 Release。
- 平台 job 失败后存在 draft：修复并重新运行同一 tag workflow；不要手工把 draft 转为公开 Release。
