# A4N · 从一行字，到一整个世界

接续 Claude Code 会话 `7579e749-082e-4495-bcf6-4eed1f2c8f21`（及续接会话 `b674caef-cc37-4746-adea-cba834c4083f`）的未完成动画。原工程位于 `work/showreel`，只实现了前 28 秒的片头、写作与章节分析。本工程保留这些场景，补齐其原定的 123 秒内容、音乐、音效和交付工具。

使用现有专用分支 `agent/project-film-20260927`。原 `work/showreel` 和分支里的 `studio/project-film` 保留；后者未作为本次创作参考。未使用已有 skill，也未以其他成片或宣传文案作为创作参考。没有修改 A4N 应用代码、项目数据、模型配置或数据库。

## 成片与工程

- `out/A4N_从一行字到一整个世界_1080p60.mp4`：1920 × 1080、60 fps、123 秒、H.264 / AAC、48 kHz 立体声，带 19 个章节标记。
- `out/A4N_动画工程与分轨.zip`：完整动画源码，以及本次的音乐、音效和混音。
- `out/music.wav`、`out/effects.wav`、`out/master.wav`：可分别编辑的音轨。
- `out/storyboard.jpg`：从最终 MP4 解码抽取的场景总览。
- `out/validation.json`、`out/review/source-check.json`：实际执行的媒体和源工程检查。
- `watch.html`：带章节跳转、原生播放控件和下载链接的本机播放器。

影片用音乐、音效和画面内中文讲解推进，没有另加人声旁白。界面、角色及模型生成结果均为动画示意，片中有标注；实际模型效果取决于所选服务。原创演示故事继续使用目标会话中的《潮汐之灯》，不是用户已有的小说或实际生成记录。

## 内容

| 时间 | 场景 |
| --- | --- |
| 0–12 秒 | 一行文字碎成粒子，再汇聚为 A4N 标志 |
| 12–28 秒 | 写作台、章节分析、档案与原文出处 |
| 28–38 秒 | 前文章节检索与上下文预算 |
| 38–48 秒 | 差异预览、作者确认与旧建议停止写回 |
| 48–72 秒 | 灯塔夜景、插画、漫画、镜头串联与互动故事 |
| 72–94 秒 | 来源快照、三端共享能力与可恢复工作流 |
| 94–112 秒 | 准备环境、启动、模型设置、开始创作 |
| 112–123 秒 | 品牌收束与仓库地址 |

功能名称和顺序沿原会话的源码核对，并复核 `debug 5237710f0` 的设置文案、CLI 命令、来源同步与上下文读取。主要依据：`frontend/src/i18n/messages/zh-CN/settings.ts`、`novel-overview.ts`、`crates/host/a4n-cli/src/cli/command_definitions.rs`、`output.rs`、`crates/host/a4n-orchestrator/src/context/rag.rs`。片中安装命令对应仓库现有脚本。

## 重现

需要 Node.js 22+、Python 3.11+、FFmpeg / ffprobe（含 libx264）、ImageMagick（montage）、系统 Google Chrome，以及 Noto Sans/Serif CJK SC、JetBrains Mono、DejaVu Sans 字体。Chromium 路径可在 `tools/browser.mjs` 按本机安装位置调整。

在本目录执行，不需构建或启动 A4N 后端：

```bash
npm ci
uv venv .venv
uv pip install --python .venv/bin/python -r requirements.txt
npm test
npm run audio
npm run render
npm run finish
npm run check:playback
npm run preview
```

`preview` 输出实际的 `127.0.0.1` 地址；按 Ctrl+C 关闭。它只提供本地静态文件，不读取模型密钥或连接数据库。播放器支持原生视频键盘控制；章节按钮可 Tab 聚焦、Enter 跳转，也可在窄屏使用。

生成内容都在忽略目录 `out/`。渲染按源码哈希分目录保存 300 帧的小段，命令中断后重跑会核验并复用完整小段；不完整小段不计为完成。源码变化会产生新的哈希目录，成片合成前再次检查源码和音轨时间轴，拒绝混用不同版本。

短段试渲染与单帧预览：

```bash
node tools/render.mjs --from 48 --to 52
npm run snap -- 8.15 33.8 50 58.4 69.2 119
```

## 声音与检查

配乐为本次在本地合成的 120 BPM 电子器乐：D 小调和声、立体声铺底、拨弦、钟琴旋律、低音和鼓组，48 秒进入高潮，片尾收束。189 个音效事件由场景登记，打字、点击、光束、翻页、冲击与海声均使用 NumPy / SciPy 合成，没有下载采样或使用素材库音乐。以两遍响度分析制作混音，实际 AAC 解码响度和真峰值写入 `out/validation.json`。

`npm test` 在真实 Chrome 中检查 7380 帧的场景覆盖、19 段连续时间轴、音效边界、57 个场景接点/中点、错误时间输入，以及 14 个画面正向/逆向跳转的可见 DOM/SVG 状态一致性与像素差。GPU 颜色量化和抗锯齿存在微小舍入差，检测容许每通道最多 2 级的量化误差，或平均差小于 0.04/255 且超过 8/255 的边缘像素不多于全画面约 0.005%；明显画面差异仍失败。

`finish` 检查源文件哈希、音轨帧数、声道、非有限数和削波，复核成片分辨率、帧率、帧数、时长、音画长度、章节、响度及真峰值，并完整解码视频与音频。源工程随机跳转检查不替代最终影片检查；本次交付还实际查看成片抽帧，并通过 Chrome 播放、跳转、窄屏和键盘操作复核。

播放验证使用 Chrome 默认配置，不沿用逐帧 WebGL 渲染的 GPU 强制启用参数。声音验收使用完整解码、波形和响度测量；本环境不支持主观回听。

工程与代码遵循仓库 Apache-2.0 许可。品牌标志沿用本项目自身的 SVG；字体使用系统已安装字体，不随工程包分发。
