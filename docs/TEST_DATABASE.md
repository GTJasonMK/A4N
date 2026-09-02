# 测试数据库

项目的 PostgreSQL 使用 Docker Compose 中的 `a4n-postgres` 容器。当前数据库角色如下：

- 主库：`a4n`，当前 `.env` 的 `A4N_DATABASE_URL` 指向这里；禁止对它执行测试清库或破坏性 fixture。
- 本次 CLI E2E 测试库：`a4n_cli_e2e_20260826`，专供本轮隔离 Server 和 fixture 使用，可安全清理或重建。
- 旧共享测试库：`a4n_test`，本轮不使用，避免把历史数据混入结果。

公共连接信息为 `postgres://a4n@127.0.0.1:5432/<database>`；密码只从本机配置文件读取，不写入文档、代码或日志。

当前开发机上的测试库已经创建完成。其他会话开始数据库测试前，先确认容器已启动：

```bash
./scripts/dev/dev-db.sh up
```

然后在当前 shell 设置本轮测试连接（不要写入提交的 `.env`，也不要把凭据复制到代码或日志）：

```bash
export A4N_TEST_DATABASE_URL='postgres://a4n:<本机配置中的密码>@127.0.0.1:5432/a4n_cli_e2e_20260826'
```

数据库测试默认应串行运行，因为部分测试会重建或清理共享 schema：

```bash
cargo test -p a4n-db --lib -- --test-threads=1
```

运行单个 crate 时同样保留 `--test-threads=1`。测试库可以安全清理和重建；不要对开发数据库 `a4n` 执行测试清库操作。
