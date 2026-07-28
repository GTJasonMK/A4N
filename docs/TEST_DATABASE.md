# 测试数据库

项目的 PostgreSQL 测试库使用 Docker Compose 中的 `a4n-postgres` 容器，但与开发库分开：

```text
数据库：a4n_test
用户：a4n
地址：127.0.0.1:5432
连接串：postgres://a4n:a4n@127.0.0.1:5432/a4n_test
```

当前开发机上的测试库已经创建完成。其他会话开始数据库测试前，先确认容器已启动：

```bash
./scripts/dev/dev-db.sh up
```

然后在当前 shell 设置测试连接（不要写入提交的 `.env`，也不要把凭据复制到代码或日志）：

```bash
export A4N_TEST_DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test'
```

数据库测试默认应串行运行，因为部分测试会重建或清理共享 schema：

```bash
cargo test -p a4n-db --lib -- --test-threads=1
```

运行单个 crate 时同样保留 `--test-threads=1`。测试库可以安全清理和重建；不要对开发数据库 `a4n` 执行测试清库操作。

