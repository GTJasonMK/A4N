# 本地 Qdrant release（A4N RAG 索引）

本目录存放 **官方 Qdrant release 二进制** 与本地数据，供开发/测试不依赖 Docker 镜像拉取。

| 路径 | 说明 |
|------|------|
| `bin/qdrant` | 可执行文件（约 85MB，默认不进 git） |
| `VERSION` | 当前版本，如 `v1.18.3` |
| `config/config.yaml` | 本机监听 `127.0.0.1:6333` / `6334` |
| `storage/` | 运行时数据（勿提交） |
| `snapshots/` | 快照目录（勿提交） |

## 安装 / 更新

```bash
./scripts/dev/setup-qdrant.sh
# 指定版本：
A4N_QDRANT_VERSION=v1.18.3 ./scripts/dev/setup-qdrant.sh
```

当前机器已下载：**v1.18.3** `x86_64-unknown-linux-gnu`。

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

## 与 Docker 的关系

`docker-compose.yml` 仍提供 `qdrant` 服务；网络不便拉镜像时用本目录 release 即可。两者不要同时占用 6333/6334。
