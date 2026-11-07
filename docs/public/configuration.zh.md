# 配置与本地数据

[README](../../README.md) · [首次运行](getting-started.zh.md) · [English](configuration.en.md)

配置项的完整模板见 [`.env.example`](../../.env.example)。真实密钥只保存在被忽略的环境文件、运行配置或应用提供的设置中。浏览器与桌面的模型配置路径可能不同，按对应设置页提示保存。

## 模型与检索

| 能力 | 配置入口与注意事项 |
| --- | --- |
| 文本、分析、Agent | 设置中的 LLM 服务；模型、接口地址、密钥、上下文容量与超时必须符合 Provider 能力 |
| Embedding | 本机设置中的嵌入模型；管理员可另设服务器默认。创建后选中并设为默认，远程模型的维度必须匹配 |
| 图片 | 生图模型设置；不同后端的图像输入与输出能力不同 |
| 视频 | 视频生成设置；按实际服务配置分辨率、时长范围与输入模式 |
| RAG | Qdrant 地址、collection，以及与项目索引匹配的 Embedding 身份 |

桌面端自带本机 fastembed 能力，无需追加 `--full`。桌面连接本地或远程 Server 时，“本机”嵌入模型由桌面管理和执行；浏览器、移动端及管理员的服务器配置使用 Server 端能力。桌面源码构建和运行都需要项目固定版本的 ONNX Runtime，准备方式见[首次运行](getting-started.zh.md#2-安装运行依赖并启动)。

`A4N_PUBLIC_LLM_*`、`A4N_PUBLIC_IMAGE_*`、`A4N_PUBLIC_VIDEO_*` 可为服务器提供公共模型配置，具体字段见模板。个人密钥和本机覆盖值按设置页的作用域保存；不要将服务端密钥放进 `VITE_*` 变量或公开示例。

文本、Embedding、图片、视频请求分别由对应 Provider 处理。采用外部服务时，相应正文、检索输入或媒体会离开本机。服务费用、内容处理和模型许可按该服务条款执行；“本地优先”不是对所有 Provider 的离线或隐私保证。

## 运行地址与隔离

| 变量 | 用途 |
| --- | --- |
| `A4N_DATABASE_URL` | PostgreSQL 连接串，桌面与 Web 都需要 |
| `A4N_JWT_SECRET` | 运行时 JWT secret |
| `A4N_BOOTSTRAP_ADMIN_USERNAME/EMAIL/PASSWORD` | 空数据库首次创建管理员，三项必须同时提供 |
| `A4N_WEB_ADDR` | Server 监听地址；桌面本地 Server 必须使用 loopback |
| `VITE_PORT` | 开发前端端口，默认 `5173` |
| `A4N_POSTGRES_PORT` | 开发 Compose 数据库暴露端口，默认 `5432` |
| `A4N_FRONTEND_DIST` | Server 托管的前端构建目录 |
| `A4N_QDRANT_URL` | Qdrant gRPC 地址，项目模板为 `http://127.0.0.1:6334`；不是 HTTP 管理端口 |
| `A4N_QDRANT_COLLECTION` | 当前环境使用的物理 collection |
| `A4N_ONNXRUNTIME_ROOT` | 匹配固定版本的完整 ONNX Runtime 安装目录 |

桌面默认 local-server 模式在未设置监听地址时会选择可用端口。显式地址保持固定，冲突时明确报错；`127.0.0.1:0` 可请求系统分配。Web 部署需要配置固定地址以配合反向代理。

多工作树环境为每个实例分配独立数据库、端口、Qdrant collection 和数据路径。不要让一次性检查连接正在创作的数据库，也不要在后台 watcher 运行时更新稳定调试基线。

## 文件与备份

| 内容 | 位置与处理方式 |
| --- | --- |
| 章节、档案、任务等结构化数据 | `A4N_DATABASE_URL` 指定的 PostgreSQL |
| 媒体与运行文件 | `A4N_DATA_DIR`，源码本地默认目录为 `data/` |
| 缓存 / 临时文件 / 日志 | `A4N_CACHE_DIR` / `A4N_TEMP_DIR` / `A4N_LOG_DIR` |
| ONNX 运行库 | 默认 `~/.local/share/a4n/onnxruntime/<version>`；由固定版本安装脚本管理 |
| Qdrant 索引 | 配置的 Qdrant 实例及 collection |

正文和档案不是存放在 `data/` 下的 SQLite 数据库。只复制媒体目录不能备份完整项目；只导出数据库也不能保住文件资产。备份应覆盖数据库、媒体及相关配置，并验证恢复。项目导入/导出仅覆盖对应功能支持的范围。

密钥、OAuth secret、日志与媒体原件不提交到 Git。README 里的精选演示素材是独立的公开文档资产，不是运行数据目录。

## 无前端 CLI

`a4n` 连接 Server 的共享 Host API，提供命令发现、JSON/JSONL 输出、危险操作授权和 task/workflow 观察。命令语义以 `--help` 和 Host manifest 为准。

先准备数据库与运行环境，将 `A4N_DATABASE_URL` 和 `A4N_JWT_SECRET` 等配置导出到当前进程环境；直接调用 Cargo 不经过 `scripts/run.sh` 的 `.env` 加载。然后从源码工作树运行：

```bash
./scripts/cargo.sh run -p a4n-cli -- local start --full
./scripts/cargo.sh run -p a4n-cli -- --output agent-json capabilities check
./scripts/cargo.sh run -p a4n-cli -- local stop
```

`local start --full` 用于源码 workspace，会构建启用 embedded capabilities 的 Server。CLI 先运行独立 migration，再启动 Server，不会自动启动 PostgreSQL 或 Docker。普通 `local start` 使用 catalog 声明的 managed sidecars。

已经自行构建或打包 CLI 时，可以直接使用：

```bash
a4n health
a4n --output agent-json capabilities export
a4n config runtime set --input @runtime-config.json
```

`config runtime show/set/clear` 管理与 GUI 本机 Provider 设置对应的运行配置。浏览器的本机设置不会自动同步到 CLI；在 CLI 中另行保存配置，并保持文本模型、Embedding 模型与维度一致。通用 `call` 按 manifest 的 `full`、`without_secrets` 或 `none` 策略注入配置；预览请求不携带 Provider 密钥。上述命令不代表当前仓库已有正式安装包。

## Linux 桌面视频

Linux 桌面入口默认设置 `WEBKIT_GST_DISABLE_GL_SINK=1` 与 `WEBKIT_DISABLE_DMABUF_RENDERER=1`，规避部分 NVIDIA/Wayland 播放栈的拖动闪黑和暂停后缩放丢帧。已有环境值优先；内存拷贝可能增加 CPU 开销。确认目标系统播放栈修复后，可设为 `0` 验证相应渲染路径。

调试、合同检查和发布命令分别见[贡献指南](../../CONTRIBUTING.md)和[发布说明](../../GITHUB_RELEASE.md)。
