# 上传 Base64 与图片 MIME 旧兼容清理记录

日期：2026-08-16

## BFS 覆盖范围

从 Galgame 音频、图片/立绘/视频素材上传和远端临时媒体上传入口出发，按生产调用链追踪：

```text
浏览器文件选择
→ frontend readFileAsBase64
→ generated Host input / apiInvoke
→ Server command dispatch 或 Tauri Host dispatch
→ Runtime action
→ Orchestrator 上传规则
→ 图片记录 / 立绘记录 / Galgame 媒体资产 / 视频记录

CLI call --set-base64
→ 原始文件字节 STANDARD Base64
→ 相同 Host input 与后端处理链

视频 provider 本地图片
→ a4n-core::media::image_mime_from_path
→ TempMediaUploadRequest / New API data URL
→ Server 临时媒体严格 MIME 校验
```

同时横向搜索 `strip_data_url_prefix`、`readFileAsBase64`、`data_base64`、`image/jpg`、
command manifest 的文件输入说明、Server/Tauri dispatch、Runtime 上传实现和测试夹具。正式前端
在 FileReader 边界已去掉 Data URL 头，CLI `--set-base64` 直接编码原始文件字节；未发现生产调用
向上传合同提交完整 Data URL。未发现 XML、反射、动态代理、AOP、DI 容器或隐藏路由入口。

## 已执行清理

1. 图片上传只接受字段名所声明的原始 Base64，删除 Data URL 兼容解析。
2. Galgame 音频上传只接受原始 Base64，删除同一旧兼容路径。
3. 章节视频、装配插入、参考媒体视频及画布视频共用的上传源只接受原始 Base64。
4. 删除零剩余用途的 `strip_data_url_prefix` 公共 helper。
5. 删除 Orchestrator `media` 模块对该旧 helper 的公开 re-export，不保留兼容别名。
6. 图片上传规则测试不再把 Data URL 记作合法输入，只保留正式 raw Base64 合同。
7. 前端把单调用 `readFileAsDataUrl` 与 `dataUrlToBase64` 合并进唯一 `readFileAsBase64`，
   删除两层私有转发。
8. 前端图片上传白名单删除非标准 `image/jpg`，只保留 PNG、标准 JPEG 与 WebP MIME。
9. Orchestrator 图片上传边界删除 `image/jpg → image/jpeg` 兼容转换。
10. Server 临时媒体上传删除 `image/jpg → image/jpeg` 兼容转换；内部生产者只生成标准 MIME。
11. New API 首帧 data URL 校验删除 `image/jpg` 别名；本地图片 MIME 由 Core 标准映射生成。
12. 保留外部 provider HTTP 响应探测中的 `image/jpg` 接受：该值来自第三方响应边界，
    不属于本项目上传合同，删除会拒绝仍可能存在的外部 Content-Type。
13. 静态搜索确认上传链已无 `strip_data_url_prefix`、Data URL 双格式说明或内部
    `image/jpg` 接受分支。

未新增默认值、兼容、回退、占位逻辑或第二套解析器。

## 验证状态

遵照资源约束，本批只使用源码阅读、`rg`、普通 diff 与可审查补丁；未运行 Cargo、npm、
测试、构建、lint、fmt、数据库、浏览器、`git diff --check` 或任何验证脚本。
