# 测试数据库

## 隔离要求

数据库测试只连接本轮明确分配的独立 PostgreSQL 数据库。开发库 `a4n`、生产库和其他会话正在使用的测试库不能用于清表、重建 schema 或破坏性 fixture。库名中的 `test` 不是清理授权；执行前确认实际服务器、数据库和所有者。

`A4N_TEST_DATABASE_URL` 是 Rust 数据库测试的连接入口。不要照抄历史记录中的带日期库名，也不要假定其他机器或之前的会话已创建好测试库。

## 准备与执行

本地 PostgreSQL 的启动入口：

```bash
./scripts/dev/dev-db.sh up
```

创建并确认本轮专用库后，在当前 shell 配置连接串。下面是格式示例，需替换为本机测试角色与凭据：

```bash
export A4N_TEST_DATABASE_URL='postgres://TEST_USER:TEST_PASSWORD@127.0.0.1:5432/a4n_test_example'
timeout 60s ./scripts/cargo.sh test -p a4n-db --lib -- --test-threads=1
```

涉及 schema 或共享表的测试必须串行；并行会话各自使用不同数据库。前述命令只设置测试连接，不负责创建数据库。测试数据库和凭据不写入仓库，也不在日志中输出完整连接串。

部分测试在环境变量缺失时会提前返回，因此进程退出为零不一定说明数据库路径已被验证。记录连接环境、测试筛选条件、结果及跳过项；不要把编译完成或超时写成测试通过。

## 脚本差异

- `scripts/test.sh` 只支持 `--crate` 和 `--nocapture`，不能直接追加任意 Cargo/test 参数；需要筛选或 `--test-threads=1` 时使用上面的 `scripts/cargo.sh`。
- 需要 ONNX Runtime 的 crate 先按 [README](../README.md)准备依赖；不要为了绕过环境检查修改测试语义。
- CLI、浏览器和 export Worker 的集成测试还需要各自的 Server、端口和数据目录。具体要求见 [CLI E2E](virtual-provider-cli-e2e.md)、[浏览器验收](browser-full-project-e2e.md)和 [CLI 测试指南](../crates/a4n-cli/TEST.md)。
- 只有在确认没有其他使用者、且属于本轮可销毁 fixture 时，才回收该测试数据库。
