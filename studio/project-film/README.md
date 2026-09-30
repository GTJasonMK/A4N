# A4N · 想象，有了作品的形状

本次从零创作的 120 秒项目介绍动画。1920 × 1080，30 fps，H.264 / AAC，48 kHz 立体声。配有中文合成旁白、逐句字幕、原创电子配乐和交互/转场音效。未使用已有 skill、旧视频、旧宣传文案或素材库音乐。

原创演示故事：**最后一座灯塔，收到来自明天的求救。** 一束光贯穿灵感、书页、蓝图、正文、镜头与派生作品。

## 文件

- `out/A4N_项目介绍_1080p.mp4`：含混音、画面内字幕与可选软字幕轨的完整成片。
- `out/A4N_中文字幕.srt`：独立中文字幕。
- `out/music.wav`、`effects.wav`、`narration.wav`：独立配乐、音效、旁白音轨。
- `out/master.wav`：完整混音；目标响度 −16 LUFS，真峰值 −1.5 dBTP。
- `out/storyboard.jpg`：分镜总览。
- `film.html`：确定性 Canvas 动画，含透视书页、矢量场景、操作示意和字幕。
- `script.json`：原创旁白、场景顺序和时间轴。
- `produce.py`：旁白生成、配乐/音效合成、逐帧渲染和可运行的交付检查。

所有操作界面和故事画面均为动画示意，不是产品实录或模型生成效果承诺。视频突出入门主线；具体模型能力由所选服务商决定。未修改 A4N 应用源码、用户项目、模型配置或数据库。

## 内容依据

功能名称与操作顺序来自当前源码核对；没有以既有宣传内容作为创作参考。基线为 `debug d07261552c783b263dae0821ae1fc884b0ba0231`，并只读复核当前 debug 中相关入口的名称与顺序。

| 片中内容 | 项目源码依据 |
| --- | --- |
| AI 辅助创作 / 自由创作 / 视频画布项目 | `frontend/src/pages/novel-list/NovelListDialogs.tsx` |
| 服务商、API、模型、保存、测试、默认配置 | `frontend/src/pages/settings/LlmTab.tsx`、`config-list/ConfigEditorPanel.tsx` |
| 灵感对话、生成并确认蓝图 | `frontend/src/pages/inspiration/useBlueprintWorkflow.ts`、`frontend/src/pages/InspirationChat.tsx` |
| 正文与角色/世界观/伏笔 | `frontend/src/pages/writing-desk/ChapterWorkspace.tsx`、`WritingContextCards.tsx` |
| 先确认视觉编排，再生成拍摄剧本 | `frontend/src/i18n/messages/zh-CN/visual-plan.ts`、`shooting-script.ts` |
| 从文本项目选择章节并创建派生项目 | `frontend/src/i18n/messages/zh-CN/novel-project.ts`、`frontend/src/pages/VideoProjectWorkspace.tsx` |
| 文本、模型、图像、视频与成片画布 | `crates/canvas/a4n-canvas-contract/src/lib.rs`、`frontend/src/pages/video-studio/nodeContracts.ts` |
| 检查来源更新、处理差异、应用安全更新 | `frontend/src/i18n/messages/zh-CN/novel-overview.ts`、`frontend/src/pages/derived/SourceSyncDialog.tsx` |
| TXT / Markdown / EPUB / PDF 与 WebGAL 工程 | `frontend/src/pages/novel-detail/ExportPanel.tsx`、`frontend/src/i18n/messages/zh-CN/galgame.ts` |

## 重渲染

需要 Python 3.11+、NumPy、Pillow、Playwright、edge-tts、FFmpeg（libx264）、Chromium、Noto Sans/Serif CJK SC 和 DejaVu Sans/Mono 字体。生产环境使用独立虚拟环境，不修改项目的 npm/Cargo 依赖。

在本目录执行：

```bash
python produce.py audio
python produce.py preview
python produce.py render
python produce.py check
```

首次旁白生成通过 Edge TTS 的 `zh-CN-YunxiNeural` 语音服务联网合成；缓存以文案与声音配置哈希命名，缓存齐全时不重发请求。配乐采用本次原创的 D 小调/add9 和声、FM 键盘音色、琶音、低音及轻打击乐；音效也在本地合成，无外部采样。渲染阶段无需联网。

本机开发预览可运行 `python -m http.server 18761 --bind 127.0.0.1` 后打开 `http://127.0.0.1:18761/film.html`，点击播放启用音频；空格暂停/继续，左右方向键跳转 5 秒。HTTP 服务只需在预览时启动。

## 验证

`produce.py check` 验证连续场景、无重叠逐句字幕、脚本文字一致性、四条音轨长度/峰值、成片尺寸/帧率/帧数/声道/时长及完整解码。实际交付时额外查看分镜与成片抽帧，并用 Chromium 播放和跳转检查。字幕时间来自语音服务的真实词边界，混音通过旁白侧链压低音乐。

生成媒体保留在忽略目录 `out/`，不进入应用 Git 提交。工程源文件单独提交在本次 agent 工作树；无需构建或运行 A4N 后端。
