export const videoCanvas = {
  canvas: {
    category: { input: "输入节点", operation: "操作节点", probe: "观察节点", output: "输出节点" },
    node: {
      textInput: "文本输入", imageInput: "图片输入", videoInput: "视频输入", audioInput: "音频输入", renderText: "文本模板", llmText: "LLM 文本处理", generateImage: "生成图片", generateVideo: "生成视频", extractFrame: "抽取帧", trimVideo: "裁剪视频", assembleVideo: "组装视频", textProbe: "文本探针", imageProbe: "图片探针", videoProbe: "视频探针", audioProbe: "音频探针", textOutput: "文本输出", imageOutput: "图片输出", videoOutput: "视频输出", audioOutput: "音频输出",
    },
    inputMode: { textOnly: "仅提示词", firstFrame: "首帧", firstEndFrame: "首尾帧", referenceImages: "参考图" },
    dataType: { text: "文本", image: "图片", video: "视频", audio: "音频", media: "媒体", json: "结构数据", any: "任意内容", imagePrompt: "图片提示词", videoPrompt: "视频提示词", shotScript: "镜头脚本", storyboard: "分镜", frame: "帧", unknown: "未知内容" },
    port: { firstFrame: "首帧", endFrame: "尾帧", referenceImage: "参考图", clip: "片段", unknown: "端口 {id}" },
    reference: { image: "图片", video: "视频", media: "媒体", text: "文本", resource: "资源" },
    imported: { image: "导入图片 #{id}", video: "导入视频 #{id}", audio: "导入音频 #{id}" },
    edgeRole: { context: "上下文", firstFrame: "首帧", endFrame: "尾帧", styleReference: "参考图", sourceVideo: "源视频", assemblyClip: "片段", finalVideo: "最终视频", connection: "连线" },
    state: { ready: "就绪", running: "运行", completed: "完成", failed: "失败", missing: "缺失", runRunning: "运行中", runCompleted: "已完成" },
    body: {
      waitingConfig: "等待配置", renderText: "按模板生成文本", missingOutputType: "缺少输出类型", generateImage: "接收上游提示词生成图片", inputMode: "输入模式：{mode}", missingInputMode: "缺少输入模式", extractFrame: "抽帧：{kind}", trim: "裁剪：{start}s–{end}s", trimDefault: "裁剪：尚未配置", assembleVideo: "接收视频片段并输出视频", textProbe: "观察并透传文本", imageProbe: "观察并透传图片", videoProbe: "观察并透传视频", audioProbe: "观察并透传音频", observed: "已观察：{type}", bound: "已绑定{type}",
    },
    issue: { missingAsset: "引用资产缺失", missingMediaReference: "缺少媒体资产引用" },
    inspector: {
      input: "输入", parameters: "参数", output: "输出", diagnostics: "诊断信息", nodeId: "节点编号", nodeType: "节点类型", missingContract: "节点合同缺失：{kind}", inputNodeHint: "该节点是输入源，不需要上游输入。", defaultOutput: "默认输出", requiredInput: "缺少必需输入", notConnected: "未连接", reference: "引用", linked: "已关联", bound: "已绑定", generated: "已生成", image: "图片", video: "视频", media: "媒体", file: "文件", thumbnail: "缩略图", mediaId: "媒体编号", imageId: "图片编号", videoId: "视频编号", filePath: "文件路径", thumbnailPath: "缩略图路径", outputPortsReady: "完成参数配置后显示可用输出端口。", terminalOutput: "该节点是终点节点，运行后在这里显示最终交付结果。", viewContent: "查看内容", operationOutput: "操作节点只传递运行输出；最终结果在输出节点中展示。", edgeDescription: "连接两个节点并传递运行结果", connectionType: "连接类型", sourcePort: "来源端口", targetPort: "目标端口", order: "顺序", text: "文本", title: "标题", displayMode: "展示模式", summary: "摘要", detail: "详细", template: "模板", textType: "文本类型", canvasPrompt: "画布提示词", videoText: "视频文本", imageText: "图片文本", llmConfig: "LLM 配置", instruction: "指令", outputType: "输出类型", shotScript: "镜头脚本", storyboard: "分镜", visualDescription: "视觉描述", dialogue: "对白", rhythmPlan: "节奏计划", videoConfig: "视频配置", inputMode: "输入模式", missingInputMode: "缺少输入模式", textOnly: "仅提示词", firstFrame: "首帧", firstEndFrame: "首尾帧", referenceImages: "参考图", negativePrompt: "负面提示词", width: "宽", height: "高", seconds: "秒", imageConfig: "图片配置", outputTitle: "输出标题", startSeconds: "开始秒", endSeconds: "结束秒", frameType: "抽帧类型", tailFrame: "尾帧", specifiedTime: "指定时间", timeSeconds: "时间秒", observedEmpty: "尚未观察到运行结果", duration: "时长", finishedTrack: "成片轨道", noClipEdges: "暂无片段连线", moveClipUp: "上移片段", moveClipDown: "下移片段", advancedReference: "高级引用", noConfigAvailable: "暂无可用配置", defaultConfig: "使用默认配置", unavailableConfig: "已删除或不可用配置", currentConfig: "当前：{name}", unavailableCurrentConfig: "当前配置已不可用，请重新选择。", generatedImageReference: "图片资源编号", generatedVideoReference: "视频资源编号", mediaAssetReference: "音频资源编号",
    },
    preview: {
      copyText: "复制文本", copyPath: "复制路径", copyFailed: "复制节点内容失败", copied: "已复制", title: "{name} 内容", fallbackTitle: "节点内容查看", subtitle: "查看节点当前保存或观察到的内容", loadingVideo: "正在加载本地视频文件", readVideoFailed: "无法读取本地视频文件", playbackFailed: "内置播放器无法播放该视频：{detail}", decodeFailed: "内置播放器无法解码该视频。文件已保存，可以复制路径后用本地播放器检查。", diagnostics: "诊断信息", videoGenerated: "视频已生成", type: "类型", textAsset: "文本资产", image: "图片", video: "视频", media: "媒体", file: "文件", thumbnail: "缩略图", duration: "时长", sourceNodes: "来源节点", reference: "引用", probeEmpty: "该探针尚未观察到{type}内容。请运行经过该探针的工作流后再查看。", outputEmpty: "该输出节点尚未保存{type}结果。请运行画布或运行到该输出节点后再查看。", providerHint: "该节点会调用外部模型，请使用“模型请求”查看将发送的内容。", noSavedContent: "该节点当前没有可查看的保存内容。",
    },
    workflow: {
      resumeFromIncomplete: "从未完成节点继续运行", cannotResume: "该运行当前不可继续", refresh: "刷新运行记录", resume: "继续运行", stop: "停止运行", ariaLabel: "工作流运行记录", status: "运行状态", loadingLatest: "正在加载最近运行", notRun: "尚未运行", discovering: "正在发现运行记录，找到后会自动显示节点进度。", noCanvasRun: "尚未运行画布节点", loadHistory: "加载运行记录", waitingProvider: "视频服务正在生成，请稍候", staleProvider: "。该运行已长时间无进度更新，可能是旧版本遗留的挂起任务；请重新运行画布生成新的后台轮询。", completed: "已完成 {completed} / {total}", modelCalls: "模型调用 {count}", noEvents: "暂无进度事件", diagnostics: "诊断信息", runId: "运行编号", taskId: "任务编号", stepId: "步骤标识", videoId: "视频编号", providerTask: "服务任务", unavailable: "运行记录不可用", waitingStart: "等待开始", processingNode: "处理画布节点", prepare: "准备运行", validate: "检查画布", execute: "运行节点", finalize: "整理结果", generateVideo: "生成视频", waitVideo: "等待视频服务", process: "处理工作流",
    },
    validation: { missingEndpoints: "连线缺少源节点或目标节点", selfConnection: "画布不允许节点自连", incompatible: "当前节点类型或端口不能直接连接", duplicate: "相同端口之间已经存在连线", singleInput: "{port} 输入只允许一个来源", edgeMismatch: "连线与当前节点端口配置不匹配：{source}（{sourceKind}.{sourcePort}） → {target}（{targetKind}.{targetPort}）。请删除该连线后按当前端口重新连接。" },
    confirm: { title: "确认运行到此节点？", executionStart: "实际执行起点：{nodes}", upstreamStart: "上游起点：{nodes}", target: "终止节点：{name}", nodeCount: "将执行节点数：{count}", inputs: "依赖输入：{nodes}", noInputs: "依赖输入：无上游输入", body: "系统会执行该节点的全部上游依赖，并停在当前节点。", nodeSeparator: "、", moreNodes: "{nodes} 等 {count} 个节点" },
    layout: { resizeLeft: "调整左侧节点库宽度", resizeRight: "调整右侧属性栏宽度", resizeHint: "拖拽调整侧栏宽度，双击恢复默认宽度" },
    unknownNode: "未知节点",
  },
} as const;
