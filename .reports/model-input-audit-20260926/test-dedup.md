# 端到端测试去重与过时用例清理

直接在 `debug` 修改，开始基线为 `d07261552c783b263dae0821ae1fc884b0ba0231`。本轮只改测试与测试文档；保留并行会话的产品代码和其他测试修改。

## 清理结果

- 删除 `frontend/e2e/nonlinear-material-sync/resume-existing.e2e.mjs`。它依赖人工遗留的项目 ID，并重复注入运行时配置、查找按钮、切换栏目和等待进度。
- 将旧脚本的独有断言迁入 `nonlinear-material-sync.e2e.mjs`：第 7 章角色同步完整、分部归纳须确认且保留已有稀疏章纲、蓝图归纳直接形成正式版本。项目由同次测试创建；增加刷新后重读。
- 非线性测试复用 `user-flow.mjs` 的登录、按钮、标签、字段、弹窗及终态等待。删除三种自动重试：模型调用重试、同步重试、短正文重生成；失败保留给诊断。
- 修正旧“生成蓝图”定位、大纲父子标签、迁到导入页的蓝图归纳入口、候选结果与分析提示。删除“按钮可点击就代表灵感对话结束”的错误判断和为旧完成门槛补发的重复提示词。
- 资料更新列表只在超过 8 项时折叠，测试按实际显示展开后检查内容，不强制要求折叠节点存在。
- 公共进度等待明确拒绝取消/停止；沿用对终态、100% 进度和关闭控件的联合检查，防止任务标题冒充成功。
- `CREATION_E2E.md` 记录各测试的独有覆盖、旧脚本去向和非线性测试运行条件。保留五组 CLI 组成测试、旧 full-project 的 RAG/Agent/画布/导出，以及视频和 Native 专项。

## 实际验证

| 验证 | 结果 |
| --- | --- |
| `npm run test:browser-helpers` | 3 项通过：陈旧元素与传输错误、进度终态与取消、诊断失败及凭证脱敏 |
| `node --check frontend/e2e/nonlinear-material-sync/nonlinear-material-sync.e2e.mjs` | 通过 |
| 非线性真实 Chromium 定向场景 | 创建模式、自由创作手工建档 2 项通过 |
| 统一 CLI 阶段检查 | 70 项通过：14 / 19 / 8 / 18 / 11 |
| 生产前端真实 Chromium | 九阶段全部通过，含 390px 编辑、图片解码、视频采用和刷新播放 |
| 生产前端构建 | 通过；保留现有大 chunk 提示 |
| 全仓 rustfmt、`git diff --check` | 本轮执行通过 |

统一入口实际命令：

```bash
python3 scripts/tests/run_creation_e2e.py \
  --binaries-dir /home/fufu/Code/A4N/work/.merge-artifacts/integration-debug-union-20260926/worktree-data/creation-e2e/integration-union-final-1/bin \
  --postgres-image docker.m.daocloud.io/library/postgres:16 \
  --report-dir data/creation-e2e/test-dedup-20260926-second
```

二进制复用前通过工具校验源码指纹和各文件哈希。主链报告的 `success` 为 `true`、浏览器退出码为 0、`failures` 为空，记录 626 次 Mock Provider 请求。

- [主链报告](../../data/creation-e2e/test-dedup-20260926-second/report.json)
- [非线性定向报告](../../data/test-dedup-20260926/nonlinear-manual-second/report.json)
- [非线性浏览器执行日志](../../data/test-dedup-20260926/nonlinear-manual-second/wdio.log)

首次主链运行因 Docker Hub 拉取 `postgres:16` 超时而未启动，随后显式使用本机已有的 PostgreSQL 16 镜像；首次定向浏览器因临时路径超过 Chromium Unix socket 长度限制而未启动，改用 `work/` 下短临时路径后通过。两次失败证据保留，没有计为通过。

## 验证边界

非线性测试中依赖指定故事内容的 5 个模型场景未重跑，当前环境未配置其专用真实模型凭证。迁移后的归纳/同步断言已核对当前组件和合同；不能把上述 2 项定向通过写成整套非线性测试通过。

其他会话在验收期间及之后继续修改前后端，主链结果只对应报告保存的源码快照；不能据此认定最新整个 `debug` 分支已通过。两个定向场景经过当前测试脚本操作本轮隔离服务，创建自己的项目，不调用真实付费模型。

本轮 Server/Worker/Provider/Qdrant/PostgreSQL/Chromium/ChromeDriver 已停止，未停止其他会话进程。修改保留在工作区，未整体提交或推送。
