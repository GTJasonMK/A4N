# 本地 Qdrant release（A4N RAG 索引）

本目录存放 **官方 Qdrant release 二进制** 与本地数据，供开发/测试不依赖 Docker 镜像拉取。

| 路径 | 说明 |
|------|------|
| `bin/qdrant` | 可执行文件（约 85MB，默认不进 git） |
| `VERSION` | 项目固定的 Qdrant release 版本 |
| `releases.tsv` | 各平台官方 release asset 与 SHA-256 |
| `config/config.yaml` | 本机监听 `127.0.0.1:6333` / `6334` |
| `storage/` | 运行时数据（勿提交） |
| `snapshots/` | 快照目录（勿提交） |

## 安装 / 更新

```bash
./scripts/dev/setup-qdrant.sh
```

安装脚本读取 `VERSION`，按当前操作系统和架构下载对应 release。

## 启动

```bash
./scripts/dev/run-qdrant.sh
```

另开终端：

```bash
export A4N_QDRANT_URL=http://127.0.0.1:6334
# 再启动 a4n-rag-service 或 full runtime
```

健康检查：

```bash
curl -s http://127.0.0.1:6333/readyz
```
